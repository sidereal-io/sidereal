//! The instance claim: the one unauthenticated write, allowed only while no
//! administrator exists.
use super::{
    password, security_event, Kind, Outcome, Role, SecurityEvent, Source, Unavailable, User,
};
use crate::database::Database;
use std::fmt;

const UNIQUE_VIOLATION: &str = "23505";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimError {
    /// The credentials break the length rules.
    Invalid,
    /// An administrator already exists.
    Claimed,
    Unavailable,
}

impl From<Unavailable> for ClaimError {
    fn from(_: Unavailable) -> Self {
        Self::Unavailable
    }
}

impl From<sqlx::Error> for ClaimError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

/// What a claim created, and the event it recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claimed {
    pub user: User,
    pub event: SecurityEvent,
}

impl Database {
    /// Whether the instance can still be claimed. Read from PostgreSQL on
    /// every call, so a restart or a second process sees the same answer.
    pub async fn claim_open(&self) -> Result<bool, Unavailable> {
        Ok(sqlx::query_scalar(
            "SELECT NOT EXISTS (SELECT 1 FROM public.users WHERE role = 'admin')",
        )
        .fetch_one(self.pool())
        .await?)
    }

    /// Create the one administrator.
    ///
    /// A closed claim answers before hashing, so a public route spends no
    /// CPU once the instance is claimed. Two first claims racing past that
    /// check are decided by the unique index on the administrator role.
    pub async fn claim(
        &self,
        username: &str,
        password: &str,
        source: Source,
    ) -> Result<Claimed, ClaimError> {
        if !self.claim_open().await? {
            return Err(ClaimError::Claimed);
        }
        if check_username(username).is_err() || check_password(password).is_err() {
            return Err(ClaimError::Invalid);
        }
        let hash = password::hash(password).await?;
        // The row commits with the administrator or not at all: a claim
        // with no record is what the audit exists to catch.
        let mut tx = self.pool().begin().await?;
        let inserted = sqlx::query(
            "INSERT INTO public.users (username, password_hash, role, created_at) VALUES ($1, $2, $3, now())",
        )
        .bind(username)
        .bind(hash)
        .bind(Role::Admin.as_str())
        .execute(&mut *tx)
        .await;
        match inserted {
            Ok(_) => {}
            Err(sqlx::Error::Database(error))
                if error.code().as_deref() == Some(UNIQUE_VIOLATION) =>
            {
                return Err(ClaimError::Claimed);
            }
            Err(_) => return Err(ClaimError::Unavailable),
        }
        let event = SecurityEvent {
            kind: Kind::InstanceClaim,
            outcome: Outcome::Success,
            source,
            username: Some(username.to_owned()),
            sessions_ended: None,
            retry_after: None,
        };
        security_event::insert(&mut *tx, &event).await?;
        tx.commit().await?;
        Ok(Claimed {
            user: User {
                username: username.to_owned(),
                role: Role::Admin,
            },
            event,
        })
    }
}

/// The credential rule a username or password breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialRule {
    UsernameLength,
    UsernameWhitespace,
    PasswordLength,
}

impl fmt::Display for CredentialRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UsernameLength => "username must be 1 to 64 characters",
            Self::UsernameWhitespace => "username must not start or end with whitespace",
            Self::PasswordLength => "password must be 8 to 256 bytes",
        })
    }
}

impl std::error::Error for CredentialRule {}

/// A username is 1 to 64 characters with no surrounding whitespace.
pub fn check_username(username: &str) -> Result<(), CredentialRule> {
    if !(1..=64).contains(&username.chars().count()) {
        return Err(CredentialRule::UsernameLength);
    }
    if username.trim() != username {
        return Err(CredentialRule::UsernameWhitespace);
    }
    Ok(())
}

/// A password is 8 to 256 bytes.
pub fn check_password(password: &str) -> Result<(), CredentialRule> {
    if !(8..=256).contains(&password.len()) {
        return Err(CredentialRule::PasswordLength);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{access::Role, database::initialize, test_support::TestDatabase};
    use sqlx::{Connection, Executor, PgConnection};
    use std::{net::IpAddr, time::Duration};

    const PASSWORD: &str = "correct-horse-battery";

    async fn database(db: &TestDatabase) -> Database {
        initialize(
            db.options.clone(),
            Duration::from_secs(10),
            Duration::from_secs(10),
        )
        .await
        .unwrap()
    }

    async fn users(db: &TestDatabase) -> Vec<(String, String, String)> {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_as("SELECT username, password_hash, role FROM public.users ORDER BY id")
            .fetch_all(&mut conn)
            .await
            .unwrap()
    }

    async fn events(db: &TestDatabase) -> Vec<(String, String, String, Option<String>)> {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_as(
            "SELECT kind, outcome, source, username FROM public.security_events ORDER BY id",
        )
        .fetch_all(&mut conn)
        .await
        .unwrap()
    }

    fn claim_row(source: &str, username: &str) -> (String, String, String, Option<String>) {
        (
            "instance_claim".into(),
            "success".into(),
            source.into(),
            Some(username.into()),
        )
    }

    async fn refuse_events(db: &TestDatabase) {
        PgConnection::connect_with(&db.options)
            .await
            .unwrap()
            .execute(
                "CREATE FUNCTION public.refuse() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''refused''; END'; \
                 CREATE TRIGGER refuse BEFORE INSERT ON public.security_events FOR EACH STATEMENT EXECUTE FUNCTION public.refuse()",
            )
            .await
            .unwrap();
    }

    #[test]
    fn credential_rule_names_the_rule_a_username_breaks() {
        for (username, rule) in [
            ("", CredentialRule::UsernameLength),
            (&"s".repeat(65), CredentialRule::UsernameLength),
            (" sam", CredentialRule::UsernameWhitespace),
            ("sam\t", CredentialRule::UsernameWhitespace),
        ] {
            assert_eq!(check_username(username), Err(rule), "{username:?}");
        }
    }

    #[test]
    fn credential_rule_names_the_password_length() {
        for password in ["short".to_owned(), "p".repeat(257)] {
            assert_eq!(
                check_password(&password),
                Err(CredentialRule::PasswordLength)
            );
        }
    }

    #[test]
    fn credential_rule_accepts_credentials_at_the_limits() {
        for username in ["s".repeat(64), "é".repeat(64), "s".into()] {
            assert_eq!(check_username(&username), Ok(()));
        }
        for password in ["p".repeat(8), "p".repeat(256)] {
            assert_eq!(check_password(&password), Ok(()));
        }
    }

    #[test]
    fn credential_rule_text_names_the_subject_and_its_limit() {
        assert_eq!(
            CredentialRule::UsernameLength.to_string(),
            "username must be 1 to 64 characters"
        );
        assert_eq!(
            CredentialRule::UsernameWhitespace.to_string(),
            "username must not start or end with whitespace"
        );
        assert_eq!(
            CredentialRule::PasswordLength.to_string(),
            "password must be 8 to 256 bytes"
        );
    }

    #[tokio::test]
    async fn claim_is_open_without_an_administrator() {
        let db = TestDatabase::new();
        assert_eq!(database(&db).await.claim_open().await, Ok(true));
    }

    #[tokio::test]
    async fn claim_is_closed_once_an_administrator_exists() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database
            .claim("sam", PASSWORD, Source::Siderealctl)
            .await
            .unwrap();
        assert_eq!(database.claim_open().await, Ok(false));
    }

    #[tokio::test]
    async fn first_claim_creates_an_administrator() {
        let db = TestDatabase::new();
        let claimed = database(&db)
            .await
            .claim("sam", PASSWORD, Source::Siderealctl)
            .await
            .unwrap();
        assert_eq!(
            claimed.user,
            User {
                username: "sam".into(),
                role: Role::Admin
            }
        );
    }

    #[tokio::test]
    async fn claim_returns_its_event() {
        let db = TestDatabase::new();
        let source = Source::Http {
            client_address: Some(IpAddr::from([192, 168, 1, 20])),
            user_agent: Some("Firefox/140".into()),
        };
        let claimed = database(&db)
            .await
            .claim("sam", PASSWORD, source.clone())
            .await
            .unwrap();
        assert_eq!(
            claimed.event,
            SecurityEvent {
                kind: Kind::InstanceClaim,
                outcome: Outcome::Success,
                source,
                username: Some("sam".into()),
                sessions_ended: None,
                retry_after: None,
            }
        );
    }

    #[tokio::test]
    async fn claim_keeps_one_instance_claim_row() {
        let db = TestDatabase::new();
        database(&db)
            .await
            .claim("sam", PASSWORD, Source::Siderealctl)
            .await
            .unwrap();
        assert_eq!(events(&db).await, [claim_row("siderealctl", "sam")]);
    }

    #[tokio::test]
    async fn a_claim_whose_row_is_refused_creates_no_administrator() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        refuse_events(&db).await;
        assert_eq!(
            database.claim("sam", PASSWORD, Source::Siderealctl).await,
            Err(ClaimError::Unavailable)
        );
        assert!(users(&db).await.is_empty());
    }

    #[tokio::test]
    async fn claim_stores_an_argon2id_hash_of_the_password() {
        let db = TestDatabase::new();
        database(&db)
            .await
            .claim("sam", PASSWORD, Source::Siderealctl)
            .await
            .unwrap();
        let [(username, hash, role)] = &users(&db).await[..] else {
            panic!("expected one user");
        };
        assert_eq!((username.as_str(), role.as_str()), ("sam", "admin"));
        assert!(hash.starts_with("$argon2id$"));
        assert!(!hash.contains(PASSWORD));
    }

    #[tokio::test]
    async fn credentials_that_break_a_length_rule_write_nothing() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        let long_name = "s".repeat(65);
        let long_password = "p".repeat(257);
        for (case, username, password) in [
            ("short password", "sam", "short"),
            ("long password", "sam", long_password.as_str()),
            ("empty username", "", PASSWORD),
            ("long username", long_name.as_str(), PASSWORD),
            ("leading space", " sam", PASSWORD),
            ("trailing space", "sam\t", PASSWORD),
        ] {
            assert_eq!(
                database
                    .claim(username, password, Source::Siderealctl)
                    .await,
                Err(ClaimError::Invalid),
                "{case}"
            );
        }
        assert!(users(&db).await.is_empty());
    }

    #[tokio::test]
    async fn credentials_at_the_length_limits_are_accepted() {
        for (username, password) in [
            ("s".repeat(64), "p".repeat(8)),
            ("é".repeat(64), "p".repeat(256)),
        ] {
            let db = TestDatabase::new();
            assert!(database(&db)
                .await
                .claim(&username, &password, Source::Siderealctl)
                .await
                .is_ok());
        }
    }

    #[tokio::test]
    async fn a_second_claim_is_refused_whatever_its_credentials() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database
            .claim("sam", PASSWORD, Source::Siderealctl)
            .await
            .unwrap();
        for (username, password) in [("alex", PASSWORD), ("alex", "short")] {
            assert_eq!(
                database
                    .claim(username, password, Source::Siderealctl)
                    .await,
                Err(ClaimError::Claimed)
            );
        }
        assert_eq!(users(&db).await.len(), 1);
        assert_eq!(events(&db).await, [claim_row("siderealctl", "sam")]);
    }

    #[tokio::test]
    async fn concurrent_first_claims_create_one_administrator() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        let claims = (0..10).map(|n| {
            let database = database.clone();
            tokio::spawn(async move {
                database
                    .claim(&format!("user{n}"), PASSWORD, Source::Siderealctl)
                    .await
            })
        });
        let mut results = Vec::new();
        for claim in claims {
            results.push(claim.await.unwrap());
        }
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(results
            .iter()
            .all(|r| r.is_ok() || *r == Err(ClaimError::Claimed)));
        assert_eq!(users(&db).await.len(), 1);
        assert_eq!(events(&db).await.len(), 1);
    }

    #[tokio::test]
    async fn claim_is_unavailable_when_the_pool_is_closed() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database.close().await;
        assert_eq!(database.claim_open().await, Err(Unavailable));
        assert_eq!(
            database.claim("sam", PASSWORD, Source::Siderealctl).await,
            Err(ClaimError::Unavailable)
        );
    }
}
