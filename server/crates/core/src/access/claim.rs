//! The instance claim: the one unauthenticated write, allowed only while no
//! administrator exists.
use super::{password, Role, Unavailable, User};
use crate::database::Database;

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
    pub async fn claim(&self, username: &str, password: &str) -> Result<User, ClaimError> {
        if !self.claim_open().await? {
            return Err(ClaimError::Claimed);
        }
        if !follows_length_rules(username, password) {
            return Err(ClaimError::Invalid);
        }
        let hash = password::hash(password).await?;
        let inserted = sqlx::query(
            "INSERT INTO public.users (username, password_hash, role, created_at) VALUES ($1, $2, $3, now())",
        )
        .bind(username)
        .bind(hash)
        .bind(Role::Admin.as_str())
        .execute(self.pool())
        .await;
        match inserted {
            Ok(_) => Ok(User {
                username: username.to_owned(),
                role: Role::Admin,
            }),
            Err(sqlx::Error::Database(error))
                if error.code().as_deref() == Some(UNIQUE_VIOLATION) =>
            {
                Err(ClaimError::Claimed)
            }
            Err(_) => Err(ClaimError::Unavailable),
        }
    }
}

/// A username is 1 to 64 characters with no surrounding whitespace; a
/// password is 8 to 256 bytes.
fn follows_length_rules(username: &str, password: &str) -> bool {
    (1..=64).contains(&username.chars().count())
        && username.trim() == username
        && (8..=256).contains(&password.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{access::Role, database::initialize, test_support::TestDatabase};
    use sqlx::{Connection, PgConnection};
    use std::time::Duration;

    const PASSWORD: &str = "correct-horse-battery";

    async fn database(db: &TestDatabase) -> Database {
        initialize(
            db.options.clone(),
            Duration::from_secs(3),
            Duration::from_secs(3),
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

    #[tokio::test]
    async fn claim_is_open_without_an_administrator() {
        let db = TestDatabase::new();
        assert_eq!(database(&db).await.claim_open().await, Ok(true));
    }

    #[tokio::test]
    async fn claim_is_closed_once_an_administrator_exists() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database.claim("sam", PASSWORD).await.unwrap();
        assert_eq!(database.claim_open().await, Ok(false));
    }

    #[tokio::test]
    async fn first_claim_creates_an_administrator() {
        let db = TestDatabase::new();
        let user = database(&db).await.claim("sam", PASSWORD).await.unwrap();
        assert_eq!(
            user,
            User {
                username: "sam".into(),
                role: Role::Admin
            }
        );
    }

    #[tokio::test]
    async fn claim_stores_an_argon2id_hash_of_the_password() {
        let db = TestDatabase::new();
        database(&db).await.claim("sam", PASSWORD).await.unwrap();
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
                database.claim(username, password).await,
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
                .claim(&username, &password)
                .await
                .is_ok());
        }
    }

    #[tokio::test]
    async fn a_second_claim_is_refused_whatever_its_credentials() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database.claim("sam", PASSWORD).await.unwrap();
        for (username, password) in [("alex", PASSWORD), ("alex", "short")] {
            assert_eq!(
                database.claim(username, password).await,
                Err(ClaimError::Claimed)
            );
        }
        assert_eq!(users(&db).await.len(), 1);
    }

    #[tokio::test]
    async fn concurrent_first_claims_create_one_administrator() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        let claims = (0..10).map(|n| {
            let database = database.clone();
            tokio::spawn(async move { database.claim(&format!("user{n}"), PASSWORD).await })
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
    }

    #[tokio::test]
    async fn claim_is_unavailable_when_the_pool_is_closed() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database.close().await;
        assert_eq!(database.claim_open().await, Err(Unavailable));
        assert_eq!(
            database.claim("sam", PASSWORD).await,
            Err(ClaimError::Unavailable)
        );
    }
}
