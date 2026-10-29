//! Parse configuration before connecting. Never retain URLs in error values.
use sidereal_core::{
    database::DatabaseError,
    storage::{StorageError, StorageRoot},
};
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};
use std::{
    ffi::OsStr,
    fmt,
    io::Read,
    net::IpAddr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

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
        certificate_files(&options)?;
        Ok(Self {
            options,
            startup_timeout,
            migration_timeout,
        })
    }
}

fn certificate_files(options: &PgConnectOptions) -> Result<(), DatabaseError> {
    const LIMIT: u64 = 1024 * 1024;
    for (name, value) in options.to_url_lossy().query_pairs() {
        if !matches!(name.as_ref(), "sslrootcert" | "sslcert" | "sslkey") {
            continue;
        }
        let Some(path) = value.strip_prefix("file: ") else {
            continue;
        };
        let check = || -> std::io::Result<bool> {
            let metadata = std::fs::metadata(path)?;
            if !metadata.is_file() || metadata.len() > LIMIT {
                return Ok(false);
            }
            let file = std::fs::File::open(path)?;
            // One extra byte distinguishes a file that grew since metadata().
            let count = std::io::copy(&mut file.take(LIMIT + 1), &mut std::io::sink())?;
            Ok(count <= LIMIT)
        };
        if !check().unwrap_or(false) {
            return Err(DatabaseError::CertificateFile);
        }
    }
    Ok(())
}

/// Why the optional `WEB_ROOT` folder cannot be served.
#[derive(Debug, PartialEq, Eq)]
pub enum WebRootError {
    MissingFolder,
    MissingIndex,
}

impl fmt::Display for WebRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingFolder => "WEB_ROOT must name an existing folder",
            Self::MissingIndex => "WEB_ROOT must name a folder that holds index.html",
        })
    }
}

impl std::error::Error for WebRootError {}

/// The folder of built web shell files, when the operator sets one.
pub fn web_root_from_env() -> Result<Option<PathBuf>, WebRootError> {
    web_root(std::env::var_os("WEB_ROOT").as_deref())
}

/// An unset or empty value serves no files.
pub fn web_root(value: Option<&OsStr>) -> Result<Option<PathBuf>, WebRootError> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let root = Path::new(value);
    if !root.is_dir() {
        return Err(WebRootError::MissingFolder);
    }
    if !root.join("index.html").is_file() {
        return Err(WebRootError::MissingIndex);
    }
    Ok(Some(root.to_owned()))
}

/// The managed storage root, when `STORAGE_ROOT` is set.
pub fn storage_root_from_env() -> Result<Option<StorageRoot>, StorageError> {
    storage_root(std::env::var_os("STORAGE_ROOT").as_deref())
}

/// Unset runs database-only. Set but empty or relative is an error, unlike
/// `WEB_ROOT`, so a blank setting never silently disables storage checks.
pub fn storage_root(value: Option<&OsStr>) -> Result<Option<StorageRoot>, StorageError> {
    value
        .map(|value| StorageRoot::new(Path::new(value)))
        .transpose()
}

/// An origin reduced to what the origin check compares: the scheme, the
/// lowercase host, and the port, with 80 and 443 stored as none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

/// The origins named in `SIDEREAL_ALLOWED_ORIGINS`, beyond the request's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AllowedOrigins(Arc<[Origin]>);

impl AllowedOrigins {
    /// Whether `origin`, an `Origin` value or a whole `Referer`, is allowed
    /// for a request whose `Host` header is `host`. The scheme is not
    /// compared against `host`, which carries none. A value that does not
    /// parse, `null` included, is not allowed.
    pub fn admit(&self, origin: &str, host: Option<&str>) -> bool {
        let Some(origin) = url::Url::parse(origin)
            .ok()
            .and_then(|url| Origin::from_url(&url))
        else {
            return false;
        };
        let own = host.and_then(host_origin);
        own.is_some_and(|own| own.host == origin.host && own.port == origin.port)
            || self.0.contains(&origin)
    }
}

fn host_origin(host: &str) -> Option<Origin> {
    let url = url::Url::parse(&format!("http://{host}")).ok()?;
    let bare = url.username().is_empty() && url.password().is_none() && url.path() == "/";
    bare.then(|| Origin::from_url(&url)).flatten()
}

impl From<Vec<Origin>> for AllowedOrigins {
    fn from(origins: Vec<Origin>) -> Self {
        Self(origins.into())
    }
}

impl Origin {
    fn from_url(url: &url::Url) -> Option<Self> {
        let port = url
            .port_or_known_default()
            .filter(|port| ![80, 443].contains(port));
        Some(Self {
            scheme: url.scheme().to_owned(),
            host: url.host_str()?.to_ascii_lowercase(),
            port,
        })
    }
}

/// Why `SIDEREAL_ALLOWED_ORIGINS` cannot be used. The text never holds the
/// item, which may be a private host name.
#[derive(Debug, PartialEq, Eq)]
pub enum AllowedOriginsError {
    NotUnicode,
    Malformed,
}

impl fmt::Display for AllowedOriginsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotUnicode => "SIDEREAL_ALLOWED_ORIGINS must be text",
            Self::Malformed => {
                "SIDEREAL_ALLOWED_ORIGINS must be a comma-separated list of http or https \
                 origins, each a scheme and host with an optional port, and no path, \
                 trailing slash, wildcard or user name"
            }
        })
    }
}

impl std::error::Error for AllowedOriginsError {}

/// The extra allowed origins, when `SIDEREAL_ALLOWED_ORIGINS` is set.
pub fn allowed_origins_from_env() -> Result<AllowedOrigins, AllowedOriginsError> {
    allowed_origins(std::env::var_os("SIDEREAL_ALLOWED_ORIGINS").as_deref())
}

/// An unset or empty value adds no origin.
pub fn allowed_origins(value: Option<&OsStr>) -> Result<AllowedOrigins, AllowedOriginsError> {
    let Some(value) = value else {
        return Ok(AllowedOrigins::default());
    };
    let value = value
        .to_str()
        .ok_or(AllowedOriginsError::NotUnicode)?
        .trim();
    if value.is_empty() {
        return Ok(AllowedOrigins::default());
    }
    value
        .split(',')
        .map(|item| listed_origin(item.trim()).ok_or(AllowedOriginsError::Malformed))
        .collect::<Result<Vec<_>, _>>()
        .map(AllowedOrigins::from)
}

fn listed_origin(item: &str) -> Option<Origin> {
    // The parser accepts `*` in a host and reads `scheme://host/` as the
    // root path, so both are refused on the raw text.
    if item.contains('*') || item.ends_with('/') {
        return None;
    }
    let url = url::Url::parse(item).ok()?;
    let bare = matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none();
    bare.then(|| Origin::from_url(&url)).flatten()
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
    fn certificate_paths_are_checked_and_redacted() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.workspace/certificate-tests");
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let valid = root.join("valid.pem");
        std::fs::write(&valid, "test certificate bytes").unwrap();
        let large = root.join("large.pem");
        std::fs::File::create(&large)
            .unwrap()
            .set_len(1024 * 1024 + 1)
            .unwrap();
        let pipe = root.join("pipe");
        let _ = std::fs::remove_file(&pipe);
        assert!(std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap()
            .success());
        for path in [
            root.join("missing.pem"),
            root.clone(),
            pipe.clone(),
            large.clone(),
        ] {
            for key in ["sslrootcert", "sslcert", "sslkey"] {
                let mut url = url::Url::parse("postgres://localhost/db").unwrap();
                url.query_pairs_mut()
                    .append_pair(key, path.to_str().unwrap());
                let start = std::time::Instant::now();
                let error = DatabaseConfig::parse(Some(url.as_str()), None, None)
                    .err()
                    .unwrap();
                assert!(start.elapsed() < Duration::from_secs(1));
                assert_eq!(error, DatabaseError::CertificateFile);
                assert!(!error.to_string().contains(path.to_str().unwrap()));
            }
        }
        let options = PgConnectOptions::new().ssl_root_cert(&valid);
        assert_eq!(certificate_files(&options), Ok(()));
        let inline = PgConnectOptions::new()
            .ssl_root_cert_from_pem(b"inline CA".to_vec())
            .ssl_client_cert_from_pem(b"inline certificate")
            .ssl_client_key_from_pem(b"inline key");
        assert_eq!(certificate_files(&inline), Ok(()));
        std::fs::remove_file(pipe).unwrap();
        std::fs::remove_file(large).unwrap();
        std::fs::remove_file(valid).unwrap();
    }

    #[test]
    fn certificate_environment_is_checked() {
        const CHILD: &str = "SIDEREAL_CERTIFICATE_ENV_TEST";
        if std::env::var_os(CHILD).is_some() {
            let error = DatabaseConfig::parse(Some("postgres://localhost/db"), None, None)
                .err()
                .unwrap();
            assert_eq!(error, DatabaseError::CertificateFile);
            assert!(!error.to_string().contains("distinctive-missing-ca.pem"));
            return;
        }
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "config::tests::certificate_environment_is_checked",
            ])
            .env(CHILD, "1")
            .env("PGSSLROOTCERT", "distinctive-missing-ca.pem")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }

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
    #[test]
    fn web_root_is_optional_and_checked() {
        assert_eq!(web_root(None), Ok(None));
        assert_eq!(web_root(Some(OsStr::new(""))), Ok(None));
        let root = std::env::temp_dir().join(format!("sidereal-web-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let missing = web_root(Some(root.as_os_str())).unwrap_err();
        assert_eq!(missing, WebRootError::MissingFolder);
        assert!(missing.to_string().contains("WEB_ROOT"));
        std::fs::create_dir_all(root.join("index.html")).unwrap();
        let no_index = web_root(Some(root.as_os_str())).unwrap_err();
        assert_eq!(no_index, WebRootError::MissingIndex);
        assert!(no_index.to_string().contains("WEB_ROOT"));
        std::fs::remove_dir(root.join("index.html")).unwrap();
        assert_eq!(
            web_root(Some(root.as_os_str())),
            Err(WebRootError::MissingIndex)
        );
        std::fs::write(root.join("index.html"), "<!doctype html>").unwrap();
        assert_eq!(web_root(Some(root.as_os_str())), Ok(Some(root.clone())));
        std::fs::remove_dir_all(&root).unwrap();
    }
    fn origin(scheme: &str, host: &str, port: Option<u16>) -> Origin {
        Origin {
            scheme: scheme.into(),
            host: host.into(),
            port,
        }
    }

    #[test]
    fn allowed_origins_unset_or_empty_adds_none() {
        for value in [None, Some(OsStr::new("")), Some(OsStr::new("  "))] {
            assert_eq!(allowed_origins(value).unwrap(), AllowedOrigins::default());
        }
    }

    #[test]
    fn allowed_origins_list_is_normalized() {
        let parsed = allowed_origins(Some(OsStr::new(
            " https://Photos.Example:443 , http://localhost:5173,http://lan.example:80,https://alt.example:8443",
        )))
        .unwrap();
        assert_eq!(
            parsed,
            AllowedOrigins::from(vec![
                origin("https", "photos.example", None),
                origin("http", "localhost", Some(5173)),
                origin("http", "lan.example", None),
                origin("https", "alt.example", Some(8443)),
            ])
        );
    }

    #[test]
    fn allowed_origins_refuses_a_malformed_item_without_naming_it() {
        for item in [
            "https://photos.example/",
            "https://photos.example/gallery",
            "https://photos.example?x=1",
            "https://photos.example#top",
            "https://*.example",
            "*",
            "ftp://photos.example",
            "https://user:distinctive@photos.example",
            "https://distinctive@photos.example",
            "photos.example",
            "https://",
            "file:///srv",
            "https://photos.example,",
        ] {
            let error = allowed_origins(Some(OsStr::new(item))).unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains("SIDEREAL_ALLOWED_ORIGINS"),
                "{item}: {message}"
            );
            assert!(!message.contains("photos.example"), "{item}: {message}");
            assert!(!message.contains("distinctive"), "{item}: {message}");
        }
    }

    fn listed(value: &str) -> AllowedOrigins {
        allowed_origins(Some(OsStr::new(value))).unwrap()
    }

    #[test]
    fn origin_matches_the_host_header() {
        let none = AllowedOrigins::default();
        assert!(none.admit("http://localhost:5000", Some("localhost:5000")));
        assert!(!none.admit("http://localhost:5001", Some("localhost:5000")));
        assert!(!none.admit("http://evil.example", Some("localhost:5000")));
    }

    #[test]
    fn origin_matches_ignoring_case_and_default_ports() {
        let none = AllowedOrigins::default();
        for (origin, host) in [
            ("https://photos.example:443", "photos.example"),
            ("https://photos.example", "photos.example:443"),
            ("https://Photos.Example", "photos.example"),
            ("https://photos.example", "PHOTOS.example"),
            ("http://photos.example:80", "photos.example"),
        ] {
            assert!(none.admit(origin, Some(host)), "{origin} {host}");
        }
    }

    #[test]
    fn origin_matches_any_scheme_against_the_host_header() {
        let none = AllowedOrigins::default();
        assert!(none.admit("http://photos.example", Some("photos.example")));
        assert!(none.admit("https://photos.example", Some("photos.example")));
    }

    #[test]
    fn origin_matches_the_origin_part_of_a_referer() {
        let none = AllowedOrigins::default();
        assert!(none.admit(
            "http://localhost:5000/sign-in?next=/",
            Some("localhost:5000")
        ));
        assert!(!none.admit("http://evil.example/", Some("localhost:5000")));
    }

    #[test]
    fn origin_matches_a_listed_origin_on_scheme_host_and_port() {
        let list = listed("https://photos.example, http://localhost:5173");
        assert!(list.admit("https://photos.example", Some("app:5000")));
        assert!(list.admit("https://photos.example:443", None));
        assert!(list.admit("http://localhost:5173", Some("app:5000")));
        assert!(!list.admit("http://photos.example", Some("app:5000")));
        assert!(!list.admit("https://photos.example:8443", Some("app:5000")));
        assert!(!list.admit("http://localhost:5174", Some("app:5000")));
    }

    #[test]
    fn origin_matches_nothing_when_unparsable() {
        let list = listed("https://photos.example");
        for origin in [
            "null",
            "",
            "photos.example",
            "not a url",
            "file:///etc/passwd",
        ] {
            assert!(!list.admit(origin, Some("photos.example")), "{origin}");
        }
        assert!(!list.admit("http://localhost:5000", None));
        assert!(!list.admit("http://localhost:5000", Some("local host")));
    }

    #[test]
    fn storage_root_is_optional_but_never_blank() {
        use sidereal_core::storage::Problem;
        assert!(storage_root(None).unwrap().is_none());
        let root =
            std::env::temp_dir().join(format!("sidereal-storage-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (value, problem) in [
            (OsStr::new(""), Problem::NotAbsolute),
            (OsStr::new("relative/storage"), Problem::NotAbsolute),
            (root.as_os_str(), Problem::Missing),
        ] {
            assert_eq!(storage_root(Some(value)).unwrap_err().problem(), &problem);
        }
        assert!(!root.exists());
        std::fs::create_dir(&root).unwrap();
        assert_eq!(
            storage_root(Some(root.as_os_str()))
                .unwrap()
                .unwrap()
                .path(),
            root
        );
        std::fs::remove_dir(&root).unwrap();
    }
}
