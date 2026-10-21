//! PostgreSQL startup and compatibility policy owned by the domain-agnostic core.
use crate::storage::StorageError;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseError {
    MissingConfiguration,
    Configuration,
    Connection,
    Unreachable,
    LoginRejected,
    Tls,
    DatabaseMissing,
    CertificateFile,
    Permission,
    Version,
    Ownership,
    History,
    Schema,
    StartupDeadline,
    Migration,
}
impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action = match self {
            Self::MissingConfiguration => "set DATABASE_URL to a dedicated PostgreSQL 18 database",
            Self::Configuration => "check PostgreSQL URL, verified TLS mode, and positive timeout seconds",
            Self::Connection => "check service availability, credentials, and TLS certificate settings",
            Self::Unreachable => "start the PostgreSQL service or wait for it to become available",
            Self::LoginRejected => "correct the configured PostgreSQL credentials and client access rule",
            Self::Tls => "configure a trusted certificate for the database host",
            Self::DatabaseMissing => "create the configured dedicated PostgreSQL database",
            Self::CertificateFile => "check sslrootcert, sslcert, and sslkey paths",
            Self::Permission => "grant the configured role the required database, schema, and table privileges",
            Self::Version => "select PostgreSQL 18; do not downgrade existing data",
            Self::Ownership => "use a dedicated database; existing data was not adopted",
            Self::History => "use a compatible Sidereal binary; do not reset existing data",
            Self::Schema => "restore the expected dedicated Sidereal schema",
            Self::StartupDeadline => "check connectivity and competing startup locks or increase DATABASE_STARTUP_TIMEOUT_SECONDS",
            Self::Migration => "check permissions and database locks or increase DATABASE_MIGRATION_TIMEOUT_SECONDS; restart after correcting the cause",
        };
        write!(f, "{}: {action}", self.label())
    }
}
impl std::error::Error for DatabaseError {}
impl DatabaseError {
    /// Fixed readiness labels never include configuration or driver details.
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingConfiguration => "missing database configuration",
            Self::Configuration => "invalid database configuration",
            Self::Connection => "database connection failed",
            Self::Unreachable => "database unreachable",
            Self::LoginRejected => "database login rejected",
            Self::Tls => "database TLS verification failed",
            Self::DatabaseMissing => "database does not exist",
            Self::CertificateFile => "database TLS certificate file unreadable",
            Self::Permission => "database permission denied",
            Self::Version => "unsupported database version",
            Self::Ownership => "database is not an empty or owned Sidereal database",
            Self::History => "incompatible migration history",
            Self::Schema => "incompatible database schema",
            Self::StartupDeadline => "database startup deadline exceeded",
            Self::Migration => "database migration failed",
        }
    }
}
impl From<sqlx::Error> for DatabaseError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Io(ref error)
                if error
                    .get_ref()
                    .is_some_and(|inner| inner.is::<rustls::Error>()) =>
            {
                Self::Tls
            }
            sqlx::Error::Tls(_) => Self::Tls,
            sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut => Self::Unreachable,
            sqlx::Error::Database(error) => match error.code().as_deref() {
                Some(code)
                    if code.starts_with("08") || matches!(code, "57P01" | "57P02" | "57P03") =>
                {
                    Self::Unreachable
                }
                Some("28P01" | "28000") => Self::LoginRejected,
                Some("3D000") => Self::DatabaseMissing,
                Some("42501") => Self::Permission,
                _ => Self::Connection,
            },
            _ => Self::Connection,
        }
    }
}

/// A startup failure from either the database or managed storage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupError {
    Database(DatabaseError),
    Storage(StorageError),
}
impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for StartupError {}
impl From<DatabaseError> for StartupError {
    fn from(error: DatabaseError) -> Self {
        Self::Database(error)
    }
}
impl From<StorageError> for StartupError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

mod pairing;
mod startup;
pub use startup::{
    initialize, initialize_with_storage, Cause, Change, Database, Probe, ProbeResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_messages_name_safe_recovery_actions() {
        for (error, phrase) in [
            (DatabaseError::Unreachable, "database unreachable"),
            (DatabaseError::LoginRejected, "database login rejected"),
            (DatabaseError::Tls, "database TLS verification failed"),
            (DatabaseError::DatabaseMissing, "database does not exist"),
            (
                DatabaseError::CertificateFile,
                "database TLS certificate file unreadable",
            ),
        ] {
            let message = error.to_string();
            assert!(message.starts_with(&format!("{phrase}: ")));
            assert!(!message.split_once(": ").unwrap().1.is_empty());
        }
    }

    #[test]
    fn connection_errors_are_classified_by_type() {
        let tls = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            rustls::Error::NoCertificatesPresented,
        );
        assert_eq!(
            DatabaseError::from(sqlx::Error::Io(tls)),
            DatabaseError::Tls
        );
        for kind in [
            std::io::ErrorKind::ConnectionRefused,
            std::io::ErrorKind::InvalidData,
        ] {
            assert_eq!(
                DatabaseError::from(sqlx::Error::Io(std::io::Error::from(kind))),
                DatabaseError::Unreachable
            );
        }
        assert_eq!(
            DatabaseError::from(sqlx::Error::PoolTimedOut),
            DatabaseError::Unreachable
        );
    }
}
