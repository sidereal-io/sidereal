//! Parse configuration before connecting. Never retain URLs in error values.
use sidereal_core::database::DatabaseError;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};
use std::{net::IpAddr, str::FromStr, time::Duration};

pub struct DatabaseConfig {
    pub options: PgConnectOptions,
    pub startup_timeout: Duration,
    pub migration_timeout: Duration,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, DatabaseError> {
        Self::parse(
            std::env::var("DATABASE_URL").ok().as_deref(),
            std::env::var("DATABASE_STARTUP_TIMEOUT_SECONDS")
                .ok()
                .as_deref(),
            std::env::var("DATABASE_MIGRATION_TIMEOUT_SECONDS")
                .ok()
                .as_deref(),
        )
    }

    pub fn parse(
        value: Option<&str>,
        startup: Option<&str>,
        migration: Option<&str>,
    ) -> Result<Self, DatabaseError> {
        let startup_timeout = seconds(startup)?;
        let migration_timeout = seconds(migration)?;
        let value = value
            .filter(|s| !s.is_empty())
            .ok_or(DatabaseError::MissingConfiguration)?;
        let url = url::Url::parse(value).map_err(|_| DatabaseError::Configuration)?;
        if !matches!(url.scheme(), "postgres" | "postgresql") {
            return Err(DatabaseError::Configuration);
        }
        let mut hosts = Vec::new();
        let mut modes = Vec::new();
        for (key, value) in url.query_pairs() {
            if matches!(key.as_ref(), "host" | "hostaddr") {
                hosts.extend(value.split(',').map(str::to_owned));
            }
            if matches!(key.as_ref(), "sslmode" | "ssl-mode") {
                modes.push(value.into_owned());
            }
        }
        if hosts.is_empty() {
            hosts.extend(
                url.host_str()
                    .unwrap_or("localhost")
                    .split(',')
                    .map(str::to_owned),
            );
        }
        let parsed = PgConnectOptions::from_str(value).map_err(|_| DatabaseError::Configuration)?;
        let mode = match modes.as_slice() {
            [] if hosts.iter().all(|host| local(host))
                || (hosts.len() == 1 && parsed.get_socket().is_some()) =>
            {
                PgSslMode::Disable
            }
            [] => PgSslMode::VerifyFull,
            [mode] => match mode.as_str() {
                "disable" => PgSslMode::Disable,
                "verify-ca" => PgSslMode::VerifyCa,
                "verify-full" => PgSslMode::VerifyFull,
                _ => return Err(DatabaseError::Configuration),
            },
            _ => return Err(DatabaseError::Configuration),
        };
        let options = parsed.ssl_mode(mode).disable_statement_logging();
        Ok(Self {
            options,
            startup_timeout,
            migration_timeout,
        })
    }
}

fn local(host: &str) -> bool {
    host == "localhost"
        || host.starts_with('/')
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn seconds(value: Option<&str>) -> Result<Duration, DatabaseError> {
    let seconds = match value {
        None => 30,
        Some(value) => value
            .parse::<u64>()
            .map_err(|_| DatabaseError::Configuration)?,
    };
    if seconds == 0 {
        return Err(DatabaseError::Configuration);
    }
    Ok(Duration::from_secs(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_policy() {
        for (value, expected) in [
            ("postgres://localhost/db", PgSslMode::Disable),
            ("postgres://127.0.0.2/db", PgSslMode::Disable),
            ("postgres://[::1]/db", PgSslMode::Disable),
            (
                "postgres:///db?host=/var/run/postgresql",
                PgSslMode::Disable,
            ),
            ("postgres://db/db", PgSslMode::VerifyFull),
            (
                "postgres://user@%2Fvar%2Frun%2Fpostgresql/db",
                PgSslMode::Disable,
            ),
            ("postgres://localhost,localhost/db", PgSslMode::Disable),
            ("postgres://loopback.example/db", PgSslMode::VerifyFull),
            ("postgres:///db?host=localhost,db", PgSslMode::VerifyFull),
            (
                "postgres:///db?host=localhost,127.0.0.1",
                PgSslMode::Disable,
            ),
            ("postgres://db/db?sslmode=disable", PgSslMode::Disable),
            ("postgres://db/db?sslmode=verify-ca", PgSslMode::VerifyCa),
            (
                "postgres://localhost/db?sslmode=verify-full",
                PgSslMode::VerifyFull,
            ),
        ] {
            assert_eq!(
                std::mem::discriminant(
                    &DatabaseConfig::parse(Some(value), None, None)
                        .unwrap()
                        .options
                        .get_ssl_mode()
                ),
                std::mem::discriminant(&expected)
            );
        }
        for mode in ["allow", "prefer", "require", "invalid"] {
            assert!(DatabaseConfig::parse(
                Some(&format!("postgres://db/db?sslmode={mode}")),
                None,
                None
            )
            .is_err());
        }
        assert!(DatabaseConfig::parse(
            Some("postgres://db/db?sslmode=disable&ssl-mode=require"),
            None,
            None
        )
        .is_err());
    }
    #[test]
    fn configuration_fails_before_connections() {
        assert!(matches!(
            DatabaseConfig::parse(None, None, None),
            Err(DatabaseError::MissingConfiguration)
        ));
        for value in ["bad-distinctive-password", "http://secret.example/db"] {
            assert!(DatabaseConfig::parse(Some(value), None, None).is_err());
        }
        for value in ["0", "-1", "", "1.5", "invalid"] {
            assert!(
                DatabaseConfig::parse(Some("postgres://localhost/db"), Some(value), None).is_err()
            );
            assert!(
                DatabaseConfig::parse(Some("postgres://localhost/db"), None, Some(value)).is_err()
            );
        }
        let config = DatabaseConfig::parse(Some("postgres://localhost/db"), None, None).unwrap();
        assert_eq!(config.startup_timeout, Duration::from_secs(30));
        assert_eq!(config.migration_timeout, Duration::from_secs(30));
    }
}
