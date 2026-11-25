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
    ready_with(db, storage, Stdio::null())
}

/// [`ready`], keeping what the server writes to stderr for [`stderr`].
fn ready_logging(db: &TestDatabase, storage: &Path) -> Server {
    ready_with(db, Some(storage), Stdio::piped())
}

/// Stop the server and answer everything it wrote to stderr.
fn stderr(mut server: Server) -> String {
    let _ = server.0.kill();
    let _ = server.0.wait();
    let mut output = String::new();
    server
        .0
        .stderr
        .take()
        .expect("started with ready_logging")
        .read_to_string(&mut output)
        .unwrap();
    output
}

fn ready_with(db: &TestDatabase, storage: Option<&Path>, stderr: Stdio) -> Server {
    let port = free_port();
    let mut child = Server(
        server(db, port, storage)
            .stdout(Stdio::null())
            .stderr(stderr)
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

/// Run the server with `STORAGE_ROOT` and `WEB_ROOT` set to a failed exit,
/// and check it never bound HTTP.
fn refused_with_web_root(db: &TestDatabase, storage: &Path, web_root: &Path) -> String {
    let port = free_port();
    let Output { status, stderr, .. } = server(db, port, Some(storage))
        .env("WEB_ROOT", web_root)
        .output()
        .unwrap();
    assert!(!status.success());
    assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    String::from_utf8_lossy(&stderr).into_owned()
}

/// A web folder that holds `index.html`.
fn web_folder(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    std::fs::write(path.join("index.html"), "<!doctype html>").unwrap();
}

fn assert_overlap_refused(stderr: &str) {
    assert!(stderr.contains("STORAGE_ROOT"), "{stderr}");
    assert!(stderr.contains("WEB_ROOT"), "{stderr}");
}

#[tokio::test]
async fn web_root_overlap_refuses_storage_inside_the_web_root() {
    let db = TestDatabase::new();
    let dir = Dir::new("overlap-inside");
    let web = dir.0.join("web");
    web_folder(&web);
    std::fs::create_dir(web.join("storage")).unwrap();
    assert_overlap_refused(&refused_with_web_root(&db, &web.join("storage"), &web));
    assert_eq!(application_tables(&db).await, 0);
    assert!(std::fs::read_dir(web.join("storage"))
        .unwrap()
        .next()
        .is_none());
}

#[tokio::test]
async fn web_root_overlap_refuses_a_web_root_inside_storage() {
    let db = TestDatabase::new();
    let dir = Dir::new("overlap-holds");
    let storage = dir.0.join("storage");
    web_folder(&storage.join("web"));
    assert_overlap_refused(&refused_with_web_root(&db, &storage, &storage.join("web")));
    assert_eq!(application_tables(&db).await, 0);
}

#[tokio::test]
async fn web_root_overlap_refuses_storage_linked_to_the_web_root() {
    let db = TestDatabase::new();
    let dir = Dir::new("overlap-link");
    let web = dir.0.join("web");
    web_folder(&web);
    std::os::unix::fs::symlink(&web, dir.0.join("storage")).unwrap();
    assert_overlap_refused(&refused_with_web_root(&db, &dir.0.join("storage"), &web));
    assert_eq!(application_tables(&db).await, 0);
}

#[tokio::test]
async fn web_root_overlap_lets_separate_folders_start() {
    let db = TestDatabase::new();
    let dir = Dir::new("overlap-separate");
    let (web, storage) = (dir.0.join("web"), dir.0.join("storage"));
    web_folder(&web);
    std::fs::create_dir(&storage).unwrap();
    let port = free_port();
    let mut child = Server(
        server(&db, port, Some(&storage))
            .env("WEB_ROOT", &web)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    while !get(port, "/readyz").is_some_and(|r| r.ends_with("{\"status\":\"ready\"}")) {
        assert!(child.0.try_wait().unwrap().is_none(), "the server exited");
        assert!(Instant::now() < deadline, "server never became ready");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(stored_id(&db).await.is_some());
}

/// A paired storage folder whose `tmp` holds `files` regular files and,
/// when `kept` is set, a folder named `kept` that holds a file.
fn leftovers(db: &TestDatabase, name: &str, files: usize, kept: bool) -> Dir {
    let storage = Dir::new(name);
    drop(ready(db, Some(&storage.0)));
    let tmp = storage.0.join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    for i in 0..files {
        std::fs::write(tmp.join(format!("leftover-{i}")), b"left").unwrap();
    }
    if kept {
        std::fs::create_dir(tmp.join("kept")).unwrap();
        std::fs::write(tmp.join("kept").join("inside"), b"keep").unwrap();
    }
    storage
}

fn tmp_entries(storage: &Dir) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(storage.0.join("tmp"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn startup_cleanup_leaves_only_subfolders_in_tmp_by_readiness() {
    let db = TestDatabase::new();
    let storage = leftovers(&db, "cleanup-ready", 3, true);
    let marker = std::fs::read(storage.0.join(MARKER)).unwrap();
    let server = ready_logging(&db, &storage.0);
    assert_eq!(tmp_entries(&storage), ["kept"]);
    assert_eq!(
        std::fs::read(storage.0.join("tmp").join("kept").join("inside")).unwrap(),
        b"keep"
    );
    assert_eq!(std::fs::read(storage.0.join(MARKER)).unwrap(), marker);
    drop(server);
}

#[tokio::test]
async fn startup_cleanup_prints_how_many_files_it_removed() {
    let db = TestDatabase::new();
    let storage = leftovers(&db, "cleanup-count", 3, true);
    let output = stderr(ready_logging(&db, &storage.0));
    assert!(
        output
            .lines()
            .any(|line| line == "storage: removed 3 leftover staging files from tmp"),
        "{output}"
    );
}

#[tokio::test]
async fn startup_cleanup_names_the_subfolder_it_left() {
    let db = TestDatabase::new();
    let storage = leftovers(&db, "cleanup-kept", 3, true);
    let output = stderr(ready_logging(&db, &storage.0));
    assert!(
        output
            .lines()
            .any(|line| line.starts_with("storage: ") && line.contains("\"kept\"")),
        "{output}"
    );
}

#[tokio::test]
async fn startup_cleanup_of_an_empty_tmp_prints_no_count_line() {
    let db = TestDatabase::new();
    let storage = leftovers(&db, "cleanup-empty", 0, false);
    let output = stderr(ready_logging(&db, &storage.0));
    assert!(
        !output
            .lines()
            .any(|line| line.starts_with("storage: removed")),
        "{output}"
    );
}

#[tokio::test]
async fn startup_cleanup_refuses_a_symlinked_tmp() {
    let db = TestDatabase::new();
    let storage = Dir::new("cleanup-link");
    drop(ready(&db, Some(&storage.0)));
    let id = stored_id(&db).await;
    let outside = Dir::new("cleanup-link-target");
    std::fs::write(outside.0.join("file"), b"keep").unwrap();
    std::os::unix::fs::symlink(&outside.0, storage.0.join("tmp")).unwrap();
    let stderr = refused(&db, &storage.0);
    assert!(stderr.contains("the tmp folder"), "{stderr}");
    assert_eq!(
        std::fs::read_link(storage.0.join("tmp")).unwrap(),
        outside.0
    );
    assert_eq!(outside.entries(), ["file"]);
    assert_eq!(std::fs::read(outside.0.join("file")).unwrap(), b"keep");
    assert_eq!(stored_id(&db).await, id);
}
