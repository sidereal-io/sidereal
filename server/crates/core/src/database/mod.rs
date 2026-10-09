//! PostgreSQL startup and compatibility policy owned by the domain-agnostic core.
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseError {
    MissingConfiguration,
    Configuration,
    Connection,
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
        f.write_str(match self {
            Self::MissingConfiguration => "missing database configuration: set DATABASE_URL to a dedicated PostgreSQL 18 database",
            Self::Configuration => "invalid database configuration: check PostgreSQL URL, verified TLS mode, and positive timeout seconds",
            Self::Connection => "database connection failed: check service availability, credentials, and TLS certificate settings",
            Self::Permission => "database permission denied: grant the configured role the required database, schema, and table privileges",
            Self::Version => "unsupported database version: select PostgreSQL 18; do not downgrade existing data",
            Self::Ownership => "database is not an empty or owned Sidereal database: use a dedicated database; existing data was not adopted",
            Self::History => "incompatible migration history: use a compatible Sidereal binary; do not reset existing data",
            Self::Schema => "incompatible database schema: restore the expected dedicated Sidereal schema",
            Self::StartupDeadline => "database startup deadline exceeded: check connectivity and competing startup locks or increase DATABASE_STARTUP_TIMEOUT_SECONDS",
            Self::Migration => "database migration failed: check permissions and database locks or increase DATABASE_MIGRATION_TIMEOUT_SECONDS; restart after correcting the cause",
        })
    }
}
impl std::error::Error for DatabaseError {}
impl From<sqlx::Error> for DatabaseError {
    fn from(error: sqlx::Error) -> Self {
        if error
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| code == "42501")
        {
            Self::Permission
        } else {
            Self::Connection
        }
    }
}

