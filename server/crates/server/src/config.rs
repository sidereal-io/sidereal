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

