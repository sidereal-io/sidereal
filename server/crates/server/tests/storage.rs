//! The server binary validates STORAGE_ROOT before PostgreSQL, pairs storage
//! before binding HTTP, and serves database-only when STORAGE_ROOT is unset.
use sqlx::{ConnectOptions, Connection, PgConnection};
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const MARKER: &str = ".sidereal-storage.json";

/// A throwaway directory under the target tree, removed when the test ends.
struct Dir(PathBuf);
impl Dir {
    fn new(name: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("storage-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn entries(&self) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A running server, killed when the test ends.
struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn server(db: &TestDatabase, port: u16, storage: Option<&Path>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sidereal-server"));
    command
        .env("DATABASE_URL", db.options.to_url_lossy().to_string())
        .env("PORT", port.to_string())
        .env("DATABASE_STARTUP_TIMEOUT_SECONDS", "3")
        .env_remove("WEB_ROOT")
        .env_remove("STORAGE_ROOT");
    if let Some(storage) = storage {
        command.env("STORAGE_ROOT", storage);
    }
    command
}

fn get(port: u16, path: &str) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    Some(response)
}

/// Start the server and wait until `/readyz` answers ready.
fn ready(db: &TestDatabase, storage: Option<&Path>) -> Server {
    let port = free_port();
    let mut child = Server(
        server(db, port, storage)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if get(port, "/readyz").is_some_and(|r| r.ends_with("{\"status\":\"ready\"}")) {
            return child;
        }
        if let Some(status) = child.0.try_wait().unwrap() {
            panic!("server exited before readiness: {status}");
        }
        assert!(Instant::now() < deadline, "server never became ready");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Run the server to a failed exit and check it never bound HTTP.
fn refused(db: &TestDatabase, storage: &Path) -> String {
    let port = free_port();
    let Output {
        status,
        stdout,
        stderr,
    } = server(db, port, Some(storage)).output().unwrap();
    assert!(!status.success());
    assert!(
        String::from_utf8_lossy(&stdout).is_empty(),
        "no listener line"
    );
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    String::from_utf8_lossy(&stderr).into_owned()
}

async fn stored_id(db: &TestDatabase) -> Option<String> {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let id = sqlx::query_scalar("SELECT storage_id::text FROM public.sidereal_metadata")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
    id
}

async fn application_tables(db: &TestDatabase) -> i64 {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let count = sqlx::query_scalar(
        "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    conn.close().await.unwrap();
    count
}

#[tokio::test]
async fn unset_storage_serves_database_only() {
    let db = TestDatabase::new();
    drop(ready(&db, None));
    assert_eq!(stored_id(&db).await, None);
}

#[tokio::test]
async fn invalid_roots_fail_before_database_writes() {
    let db = TestDatabase::new();
    let dir = Dir::new("invalid");
    std::fs::write(dir.0.join("file"), "x").unwrap();
    for storage in [
        PathBuf::new(),
        PathBuf::from("relative/storage"),
        dir.0.join("absent"),
        dir.0.join("file"),
    ] {
        let stderr = refused(&db, &storage);
        assert!(stderr.starts_with("storage "), "{stderr}");
    }
    assert!(!dir.0.join("absent").exists());
    assert!(!Path::new("relative").exists());
    assert_eq!(application_tables(&db).await, 0);
}

#[tokio::test]
async fn storage_pairs_before_serving_and_missing_markers_refuse() {
    let db = TestDatabase::new();
    let storage = Dir::new("paired");
    drop(ready(&db, Some(&storage.0)));
    assert_eq!(storage.entries(), [MARKER]);
    let marker = std::fs::read_to_string(storage.0.join(MARKER)).unwrap();
    let id = stored_id(&db).await.unwrap();
    assert!(marker.contains(&id));
    drop(ready(&db, Some(&storage.0)));
    assert_eq!(
        std::fs::read_to_string(storage.0.join(MARKER)).unwrap(),
        marker
    );

    // An empty directory where the paired store should be mounted.
    let empty = Dir::new("substitute\nforged");
    let stderr = refused(&db, &empty.0);
    assert!(stderr.contains("storage marker is missing"), "{stderr}");
    assert!(stderr.contains("substitute\\nforged"), "{stderr}");
    assert!(!stderr.contains("substitute\nforged"));
    for secret in [
        db.options.to_url_lossy().to_string(),
        "postgres://".to_owned(),
        "fixture_test@".to_owned(),
    ] {
        assert!(!stderr.contains(&secret), "credential disclosure");
    }
    assert!(empty.entries().is_empty());
    assert_eq!(stored_id(&db).await.as_deref(), Some(id.as_str()));
}
