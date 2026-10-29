//! Access recovery from the box: the administrator lookup and the password
//! reset that `sidectl` runs.
use super::{check_password, password, Unavailable};
use crate::database::Database;
use rand::{distr::Alphanumeric, RngExt};

/// What a reset changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reset {
    pub username: String,
    /// How many sessions the reset ended.
    pub sessions: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetError {
    /// The password breaks a credential rule.
    Invalid,
    /// No administrator exists to reset.
    NoAdministrator,
    Unavailable,
}

impl From<Unavailable> for ResetError {
    fn from(_: Unavailable) -> Self {
        Self::Unavailable
    }
}

impl From<sqlx::Error> for ResetError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

impl Database {
    /// The administrator's username, or `None` while the instance is unclaimed.
    pub async fn administrator(&self) -> Result<Option<String>, Unavailable> {
        Ok(
            sqlx::query_scalar("SELECT username FROM public.users WHERE role = 'admin'")
                .fetch_optional(self.pool())
                .await?,
        )
    }

    /// Replace the administrator's password and end every session.
    ///
    /// One transaction holds both, so a signed-in browser can never outlive
    /// the password it signed in with.
    pub async fn reset_admin_password(&self, password: &str) -> Result<Reset, ResetError> {
        check_password(password).map_err(|_| ResetError::Invalid)?;
        let hash = password::hash(password).await?;
        let mut tx = self.pool().begin().await?;
        let username: Option<String> = sqlx::query_scalar(
            "UPDATE public.users SET password_hash = $1 WHERE role = 'admin' RETURNING username",
        )
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(username) = username else {
            return Err(ResetError::NoAdministrator);
        };
        let sessions = sqlx::query("DELETE FROM public.sessions")
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        Ok(Reset { username, sessions })
    }
}

/// A password for an operator who left the prompt empty: 24 characters drawn
/// uniformly from the ASCII letters and digits.
pub fn generate_password() -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(24)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::initialize, test_support::TestDatabase};
    use sqlx::{Connection, Executor, PgConnection};
    use std::time::Duration;

    const PASSWORD: &str = "correct-horse-battery";
    const NEW_PASSWORD: &str = "new-horse-battery";

    async fn database(db: &TestDatabase) -> Database {
        initialize(
            db.options.clone(),
            Duration::from_secs(10),
            Duration::from_secs(10),
        )
        .await
        .unwrap()
    }

    /// A database whose administrator `sam` is signed in `sessions` times.
    async fn claimed(db: &TestDatabase, sessions: usize) -> Database {
        let database = database(db).await;
        database.claim("sam", PASSWORD).await.unwrap();
        for _ in 0..sessions {
            database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
        }
        database
    }

    async fn connection(db: &TestDatabase) -> PgConnection {
        PgConnection::connect_with(&db.options).await.unwrap()
    }

    async fn hash(db: &TestDatabase) -> String {
        sqlx::query_scalar("SELECT password_hash FROM public.users WHERE role = 'admin'")
            .fetch_one(&mut connection(db).await)
            .await
            .unwrap()
    }

    async fn session_count(db: &TestDatabase) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM public.sessions")
            .fetch_one(&mut connection(db).await)
            .await
            .unwrap()
    }

    #[test]
    fn generate_password_is_24_ascii_letters_and_digits() {
        let password = generate_password();
        assert_eq!(password.len(), 24);
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_password_differs_between_calls() {
        assert_ne!(generate_password(), generate_password());
    }

    #[test]
    fn generate_password_meets_the_password_rule() {
        assert_eq!(check_password(&generate_password()), Ok(()));
    }

    #[tokio::test]
    async fn administrator_is_none_before_the_claim() {
        let db = TestDatabase::new();
        assert_eq!(database(&db).await.administrator().await, Ok(None));
    }

    #[tokio::test]
    async fn administrator_names_the_claimed_user() {
        let db = TestDatabase::new();
        let database = claimed(&db, 0).await;
        assert_eq!(database.administrator().await, Ok(Some("sam".into())));
    }

    #[tokio::test]
    async fn reset_admin_password_makes_the_new_password_sign_in() {
        let db = TestDatabase::new();
        let database = claimed(&db, 0).await;
        database.reset_admin_password(NEW_PASSWORD).await.unwrap();
        assert!(database
            .sign_in("sam", NEW_PASSWORD)
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn reset_admin_password_refuses_the_old_password() {
        let db = TestDatabase::new();
        let database = claimed(&db, 0).await;
        database.reset_admin_password(NEW_PASSWORD).await.unwrap();
        assert!(database.sign_in("sam", PASSWORD).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reset_admin_password_ends_every_session() {
        let db = TestDatabase::new();
        let database = claimed(&db, 2).await;
        assert_eq!(
            database.reset_admin_password(NEW_PASSWORD).await,
            Ok(Reset {
                username: "sam".into(),
                sessions: 2
            })
        );
        assert_eq!(session_count(&db).await, 0);
    }

    #[tokio::test]
    async fn reset_admin_password_without_an_administrator_writes_nothing() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        assert_eq!(
            database.reset_admin_password(NEW_PASSWORD).await,
            Err(ResetError::NoAdministrator)
        );
        assert_eq!(database.administrator().await, Ok(None));
    }

    #[tokio::test]
    async fn reset_admin_password_refuses_a_short_password_before_any_change() {
        let db = TestDatabase::new();
        let database = claimed(&db, 1).await;
        let before = hash(&db).await;
        assert_eq!(
            database.reset_admin_password("short").await,
            Err(ResetError::Invalid)
        );
        assert_eq!((hash(&db).await, session_count(&db).await), (before, 1));
    }

    #[tokio::test]
    async fn reset_admin_password_keeps_the_hash_when_sessions_cannot_be_deleted() {
        let db = TestDatabase::new();
        let database = claimed(&db, 1).await;
        let before = hash(&db).await;
        connection(&db)
            .await
            .execute(
                "CREATE FUNCTION public.refuse() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''refused''; END'; \
                 CREATE TRIGGER refuse BEFORE DELETE ON public.sessions FOR EACH STATEMENT EXECUTE FUNCTION public.refuse()",
            )
            .await
            .unwrap();
        assert_eq!(
            database.reset_admin_password(NEW_PASSWORD).await,
            Err(ResetError::Unavailable)
        );
        assert_eq!((hash(&db).await, session_count(&db).await), (before, 1));
    }

    #[tokio::test]
    async fn reset_admin_password_is_unavailable_when_the_pool_is_closed() {
        let db = TestDatabase::new();
        let database = claimed(&db, 0).await;
        database.close().await;
        assert_eq!(database.administrator().await, Err(Unavailable));
        assert_eq!(
            database.reset_admin_password(NEW_PASSWORD).await,
            Err(ResetError::Unavailable)
        );
    }
}
