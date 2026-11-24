//! Sessions: an opaque random token in the browser, its SHA-256 digest in
//! PostgreSQL (ADR-015).
use super::{password, Role, Unavailable, User};
use crate::database::Database;
use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};
use std::{fmt, time::Duration};

/// A session lasts this long after its most recent use.
pub const SESSION_LIFETIME: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// A use is recorded at most this often, so a busy session costs one write
/// an hour rather than one per request.
const REFRESH_AFTER: Duration = Duration::from_secs(60 * 60);

/// The value the browser holds in its cookie. It never appears in a log.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    fn new() -> Self {
        let mut bytes = [0; 32];
        rand::fill(&mut bytes);
        Self(Base64UrlUnpadded::encode_string(&bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn digest(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(..)")
    }
}

impl Database {
    /// Start a session when the password matches, or answer `None`.
    ///
    /// An unknown username is checked against a dummy hash, so it costs the
    /// same as a wrong password and the answer does not reveal which it was.
    ///
    /// The session is saved only while the user row still holds the hash the
    /// password matched. The share lock waits for an open reset, and once it
    /// commits the replaced hash no longer matches, so no session made with
    /// the old password outlives the reset. No lock is held during the slow
    /// password check.
    pub async fn sign_in(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<(User, Token)>, Unavailable> {
        let found: Option<(i64, String, String)> =
            sqlx::query_as("SELECT id, password_hash, role FROM public.users WHERE username = $1")
                .bind(username)
                .fetch_optional(self.pool())
                .await?;
        let hash = match &found {
            Some((_, hash, _)) => hash.clone(),
            None => password::dummy().await?,
        };
        let matches = password::verify(password, hash).await?;
        let Some((id, hash, role)) = found.filter(|_| matches) else {
            return Ok(None);
        };
        let role = Role::parse(&role).ok_or(Unavailable)?;
        let token = Token::new();
        let saved = sqlx::query(
            "WITH checked AS (SELECT id FROM public.users WHERE id = $2 AND password_hash = $4 FOR SHARE) \
             INSERT INTO public.sessions (token_hash, user_id, auth_type, created_at, last_used_at, expires_at) \
             SELECT $1, id, 'password', now(), now(), now() + make_interval(secs => $3) FROM checked",
        )
        .bind(digest(token.as_str()))
        .bind(id)
        .bind(SESSION_LIFETIME.as_secs_f64())
        .bind(hash)
        .execute(self.pool())
        .await?
        .rows_affected();
        if saved == 0 {
            return Ok(None);
        }
        let user = User {
            id,
            username: username.to_owned(),
            role,
        };
        Ok(Some((user, token)))
    }

    /// The user an unexpired session belongs to.
    ///
    /// An expired session met here is deleted. A periodic sweep removes the
    /// ones nobody presents again.
    pub async fn session(&self, token: &str) -> Result<Option<User>, Unavailable> {
        let hash = digest(token);
        let found: Option<(i64, String, String, bool, bool)> = sqlx::query_as(
            "SELECT u.id, u.username, u.role, s.expires_at <= now(), s.last_used_at <= now() - make_interval(secs => $2) \
             FROM public.sessions s JOIN public.users u ON u.id = s.user_id WHERE s.token_hash = $1",
        )
        .bind(&hash)
        .bind(REFRESH_AFTER.as_secs_f64())
        .fetch_optional(self.pool())
        .await?;
        let Some((id, username, role, expired, stale)) = found else {
            return Ok(None);
        };
        if expired {
            sqlx::query("DELETE FROM public.sessions WHERE token_hash = $1")
                .bind(&hash)
                .execute(self.pool())
                .await?;
            return Ok(None);
        }
        if stale {
            sqlx::query(
                "UPDATE public.sessions SET last_used_at = now(), expires_at = now() + make_interval(secs => $2) \
                 WHERE token_hash = $1",
            )
            .bind(&hash)
            .bind(SESSION_LIFETIME.as_secs_f64())
            .execute(self.pool())
            .await?;
        }
        let role = Role::parse(&role).ok_or(Unavailable)?;
        Ok(Some(User { id, username, role }))
    }

    /// Whether a token still names an unexpired administrator session.
    ///
    /// Unlike [`Database::session`] it only reads: an open socket checking
    /// again is not a use, and an expired row is left for the guard or the
    /// sweep to delete.
    pub async fn session_alive(&self, token: &str) -> Result<bool, Unavailable> {
        let alive: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM public.sessions s JOIN public.users u ON u.id = s.user_id \
             WHERE s.token_hash = $1 AND s.expires_at > now() AND u.role = 'admin')",
        )
        .bind(digest(token))
        .fetch_one(self.pool())
        .await?;
        Ok(alive)
    }

    /// End a session. Answers whether the token named an unexpired one; an
    /// expired row is deleted either way.
    pub async fn sign_out(&self, token: &str) -> Result<bool, Unavailable> {
        let deleted: Option<bool> = sqlx::query_scalar(
            "DELETE FROM public.sessions WHERE token_hash = $1 RETURNING expires_at > now()",
        )
        .bind(digest(token))
        .fetch_optional(self.pool())
        .await?;
        Ok(deleted.unwrap_or(false))
    }

    /// Delete every expired session, and answer how many went. The test for
    /// expired is the one [`Database::session`] uses, so the two agree.
    pub async fn sweep_sessions(&self) -> Result<u64, Unavailable> {
        let deleted = sqlx::query("DELETE FROM public.sessions WHERE expires_at <= now()")
            .execute(self.pool())
            .await?
            .rows_affected();
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::{Reset, ResetError, Role, Source},
        database::initialize,
        test_support::TestDatabase,
    };
    use base64ct::{Base64UrlUnpadded, Encoding};
    use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection};
    use tokio::task::JoinHandle;

    const PASSWORD: &str = "correct-horse-battery";
    const NEW_PASSWORD: &str = "new-horse-battery";
    /// The advisory lock key a held reset waits on.
    const HOLD: i64 = 462;
    /// How long a test waits for another connection before it fails.
    const WAIT: Duration = Duration::from_secs(10);

    async fn claimed(db: &TestDatabase) -> Database {
        let database = initialize(
            db.options.clone(),
            Duration::from_secs(10),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        database
            .claim("sam", PASSWORD, Source::Siderealctl)
            .await
            .unwrap();
        database
    }

    async fn connection(db: &TestDatabase) -> PgConnection {
        PgConnection::connect_with(&db.options).await.unwrap()
    }

    async fn sessions(db: &TestDatabase) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM public.sessions")
            .fetch_one(&mut connection(db).await)
            .await
            .unwrap()
    }

    /// Run `sql` against every session row, then read one boolean from it.
    async fn on_sessions(db: &TestDatabase, sql: &'static str) -> bool {
        sqlx::query_scalar(AssertSqlSafe(sql))
            .fetch_one(&mut connection(db).await)
            .await
            .unwrap()
    }

    async fn hash(db: &TestDatabase) -> String {
        sqlx::query_scalar("SELECT password_hash FROM public.users WHERE username = 'sam'")
            .fetch_one(&mut connection(db).await)
            .await
            .unwrap()
    }

    /// Whether a backend in this test's database waits on a lock. `advisory`
    /// picks advisory locks; otherwise only the others count.
    async fn waiting_on(db: &TestDatabase, advisory: bool) -> bool {
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_locks l JOIN pg_stat_activity a ON a.pid = l.pid \
             WHERE a.datname = current_database() AND NOT l.granted AND (l.locktype = 'advisory') = $1)",
        )
        .bind(advisory)
        .fetch_one(&mut connection(db).await)
        .await
        .unwrap()
    }

    /// Poll `done` until it answers true, failing after [`WAIT`].
    async fn wait_until(mut done: impl AsyncFnMut() -> bool) {
        tokio::time::timeout(WAIT, async {
            while !done().await {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("waited too long");
    }

    /// A reset that has replaced the hash and deleted every session, and
    /// waits to commit until [`HeldReset::release`].
    struct HeldReset {
        lock: PgConnection,
        task: JoinHandle<Result<Reset, ResetError>>,
    }

    impl HeldReset {
        async fn start(db: &TestDatabase, database: &Database) -> Self {
            let mut lock = connection(db).await;
            lock.execute(AssertSqlSafe(format!(
                "SELECT pg_advisory_lock({HOLD}); \
                 CREATE FUNCTION public.hold() RETURNS trigger LANGUAGE plpgsql \
                 AS 'BEGIN PERFORM pg_advisory_xact_lock_shared({HOLD}); RETURN NULL; END'; \
                 CREATE TRIGGER hold AFTER DELETE ON public.sessions \
                 FOR EACH STATEMENT EXECUTE FUNCTION public.hold()"
            )))
            .await
            .unwrap();
            let database = database.clone();
            let task =
                tokio::spawn(async move { database.reset_admin_password(NEW_PASSWORD).await });
            wait_until(async || waiting_on(db, true).await).await;
            Self { lock, task }
        }

        async fn release(mut self) -> Result<Reset, ResetError> {
            self.lock
                .execute(AssertSqlSafe(format!("SELECT pg_advisory_unlock({HOLD})")))
                .await
                .unwrap();
            tokio::time::timeout(WAIT, self.task)
                .await
                .expect("waited too long")
                .unwrap()
        }
    }

    /// The claimed administrator, with the id its row has.
    async fn sam(db: &TestDatabase) -> User {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        let id = sqlx::query_scalar("SELECT id FROM public.users WHERE username = 'sam'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        User {
            id,
            username: "sam".into(),
            role: Role::Admin,
        }
    }

    #[tokio::test]
    async fn a_held_reset_commits_only_when_released() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let before = hash(&db).await;
        let held = HeldReset::start(&db, &database).await;
        assert_eq!(hash(&db).await, before);
        assert!(!held.task.is_finished());
        assert!(held.release().await.is_ok());
        assert_ne!(hash(&db).await, before);
    }

    #[tokio::test]
    async fn a_sign_in_during_a_reset_answers_none_and_saves_nothing() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let held = HeldReset::start(&db, &database).await;
        let sign_in = tokio::spawn({
            let database = database.clone();
            async move { database.sign_in("sam", PASSWORD).await }
        });
        let mut waited = false;
        wait_until(async || {
            waited = waiting_on(&db, false).await;
            waited || sign_in.is_finished()
        })
        .await;
        assert!(held.release().await.is_ok());
        assert_eq!(sign_in.await.unwrap(), Ok(None));
        assert_eq!(sessions(&db).await, 0);
        assert!(waited, "the sign-in never waited on the reset's row lock");
    }

    #[tokio::test]
    async fn sign_in_with_the_claimed_password_returns_the_user() {
        let db = TestDatabase::new();
        let (user, _) = claimed(&db)
            .await
            .sign_in("sam", PASSWORD)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user, sam(&db).await);
    }

    #[tokio::test]
    async fn a_token_encodes_32_bytes() {
        let db = TestDatabase::new();
        let (_, token) = claimed(&db)
            .await
            .sign_in("sam", PASSWORD)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            Base64UrlUnpadded::decode_vec(token.as_str()).unwrap().len(),
            32
        );
    }

    #[tokio::test]
    async fn each_sign_in_gets_a_new_token() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, first) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        let (_, second) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn the_session_row_holds_the_sha256_of_the_token() {
        let db = TestDatabase::new();
        let (_, token) = claimed(&db)
            .await
            .sign_in("sam", PASSWORD)
            .await
            .unwrap()
            .unwrap();
        let matches: bool = sqlx::query_scalar(
            "SELECT token_hash = sha256(convert_to($1, 'UTF8')) AND auth_type = 'password' FROM public.sessions",
        )
        .bind(token.as_str())
        .fetch_one(&mut connection(&db).await)
        .await
        .unwrap();
        assert!(matches);
    }

    #[tokio::test]
    async fn a_new_session_expires_30_days_after_sign_in() {
        let db = TestDatabase::new();
        claimed(&db).await.sign_in("sam", PASSWORD).await.unwrap();
        assert!(on_sessions(&db, "SELECT bool_and(created_at = last_used_at AND expires_at = last_used_at + interval '30 days') FROM public.sessions").await);
    }

    #[tokio::test]
    async fn a_wrong_password_creates_no_session() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        assert_eq!(database.sign_in("sam", "wrong-password").await, Ok(None));
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn an_unknown_username_creates_no_session() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        assert_eq!(database.sign_in("nobody", PASSWORD).await, Ok(None));
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn known_and_unknown_usernames_both_verify_a_password() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        for username in ["sam", "nobody"] {
            password::VERIFICATIONS
                .scope(std::cell::Cell::new(0), async {
                    assert_eq!(database.sign_in(username, "wrong-password").await, Ok(None));
                    assert_eq!(
                        password::VERIFICATIONS.with(|count| count.get()),
                        1,
                        "{username}"
                    );
                })
                .await;
        }
    }

    #[tokio::test]
    async fn a_token_names_its_user() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        assert_eq!(
            database.session(token.as_str()).await,
            Ok(Some(sam(&db).await))
        );
    }

    #[tokio::test]
    async fn an_unknown_token_names_nobody() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        database.sign_in("sam", PASSWORD).await.unwrap();
        assert_eq!(database.session("not-a-token").await, Ok(None));
    }

    #[tokio::test]
    async fn an_expired_session_is_deleted_on_use() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        on_sessions(&db, "WITH moved AS (UPDATE public.sessions SET expires_at = now() - interval '1 minute' RETURNING 1) SELECT true FROM moved").await;
        assert_eq!(database.session(token.as_str()).await, Ok(None));
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn a_use_after_an_hour_moves_the_expiry() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        on_sessions(&db, "WITH moved AS (UPDATE public.sessions SET last_used_at = now() - interval '29 days', expires_at = now() + interval '1 day' RETURNING 1) SELECT true FROM moved").await;
        assert_eq!(
            database.session(token.as_str()).await,
            Ok(Some(sam(&db).await))
        );
        assert!(on_sessions(&db, "SELECT bool_and(last_used_at > now() - interval '1 minute' AND expires_at >= now() + interval '30 days' - interval '1 minute') FROM public.sessions").await);
    }

    #[tokio::test]
    async fn a_use_within_the_hour_writes_nothing() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        on_sessions(&db, "WITH moved AS (UPDATE public.sessions SET last_used_at = now() - interval '30 minutes', expires_at = now() + interval '1 day' RETURNING 1) SELECT true FROM moved").await;
        database.session(token.as_str()).await.unwrap();
        assert!(
            on_sessions(
                &db,
                "SELECT bool_and(expires_at < now() + interval '2 days') FROM public.sessions"
            )
            .await
        );
    }

    #[tokio::test]
    async fn session_alive_answers_true_for_an_unexpired_session() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        assert_eq!(database.session_alive(token.as_str()).await, Ok(true));
    }

    #[tokio::test]
    async fn session_alive_answers_false_for_a_missing_session() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        database.sign_in("sam", PASSWORD).await.unwrap();
        assert_eq!(database.session_alive("not-a-token").await, Ok(false));
    }

    #[tokio::test]
    async fn session_alive_answers_false_for_an_expired_session_and_keeps_its_row() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        on_sessions(&db, "WITH moved AS (UPDATE public.sessions SET expires_at = now() - interval '1 minute' RETURNING 1) SELECT true FROM moved").await;
        assert_eq!(database.session_alive(token.as_str()).await, Ok(false));
        assert_eq!(sessions(&db).await, 1);
    }

    #[tokio::test]
    async fn session_alive_records_no_use() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        on_sessions(&db, "WITH moved AS (UPDATE public.sessions SET last_used_at = now() - interval '2 days', expires_at = now() + interval '28 days' RETURNING 1) SELECT true FROM moved").await;
        assert_eq!(database.session_alive(token.as_str()).await, Ok(true));
        assert!(on_sessions(&db, "SELECT bool_and(last_used_at < now() - interval '1 day' AND expires_at < now() + interval '29 days') FROM public.sessions").await);
    }

    #[tokio::test]
    async fn sign_out_deletes_the_session() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        assert_eq!(database.sign_out(token.as_str()).await, Ok(true));
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn sign_out_of_an_unknown_token_is_refused() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        assert_eq!(database.sign_out("not-a-token").await, Ok(false));
    }

    #[tokio::test]
    async fn a_sweep_deletes_only_expired_sessions() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        database.sign_in("sam", PASSWORD).await.unwrap();
        database.sign_in("sam", PASSWORD).await.unwrap();
        on_sessions(&db, "WITH moved AS (UPDATE public.sessions SET expires_at = now() - interval '1 minute' RETURNING 1) SELECT true FROM moved").await;
        let (_, token) = database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        assert_eq!(database.sweep_sessions().await, Ok(2));
        assert_eq!(sessions(&db).await, 1);
        assert_eq!(database.session_alive(token.as_str()).await, Ok(true));
    }

    #[tokio::test]
    async fn a_sweep_with_nothing_expired_deletes_nothing() {
        let db = TestDatabase::new();
        let database = claimed(&db).await;
        database.sign_in("sam", PASSWORD).await.unwrap();
        database.sign_in("sam", PASSWORD).await.unwrap();
        assert_eq!(database.sweep_sessions().await, Ok(0));
        assert_eq!(sessions(&db).await, 2);
    }

    #[tokio::test]
    async fn a_token_does_not_appear_in_debug_output() {
        let db = TestDatabase::new();
        let (_, token) = claimed(&db)
            .await
            .sign_in("sam", PASSWORD)
            .await
            .unwrap()
            .unwrap();
        assert!(!format!("{token:?}").contains(token.as_str()));
    }
}
