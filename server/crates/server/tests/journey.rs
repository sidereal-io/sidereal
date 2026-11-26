//! The first-image journey through real server processes: Nova uploads a real
//! PNG and a real JPEG, the server restarts with the same database and
//! storage, and the same Assets, provenance and original bytes come back.
use serde_json::Value;
use sqlx::{ConnectOptions, Connection, PgConnection, Row};
use std::{
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PNG: &[u8] = include_bytes!("fixtures/nebula.png");
const JPEG: &[u8] = include_bytes!("fixtures/nebula.jpg");
const BOUNDARY: &str = "sidereal-journey-boundary";
const MARKER: &str = ".sidereal-storage.json";

/// A throwaway folder under the target tree, removed when the test ends.
struct Dir(PathBuf);
impl Dir {
    fn new(name: &str) -> Self {
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("journey-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A running server process, killed when it is dropped.
struct Server {
    child: Child,
    port: u16,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Start the server with `STORAGE_ROOT` and no `WEB_ROOT`, and wait until
/// `/readyz` answers ready.
fn start(db: &TestDatabase, storage: &Path) -> Server {
    let port = free_port();
    let child = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
        .env("DATABASE_URL", db.options.to_url_lossy().to_string())
        .env("PORT", port.to_string())
        .env("STORAGE_ROOT", storage)
        .env("DATABASE_STARTUP_TIMEOUT_SECONDS", "3")
        .env_remove("WEB_ROOT")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = Server { child, port };
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let ready = send(&server, "GET", "/readyz", &[], b"")
            .is_ok_and(|reply| reply.body == br#"{"status":"ready"}"#);
        if ready {
            return server;
        }
        if let Some(status) = server.child.try_wait().unwrap() {
            panic!("server exited before readiness: {status}");
        }
        assert!(Instant::now() < deadline, "server never became ready");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// One answer from the server, with its body decoded from chunks.
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap()
    }
}

/// One HTTP/1.1 request on its own connection.
fn send(
    server: &Server,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> std::io::Result<Reply> {
    let mut stream = TcpStream::connect(("127.0.0.1", server.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\nContent-Length: {}\r\n",
        server.port,
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;

    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("no end of headers"))?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| std::io::Error::other("no status"))?;
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let mut reply = Reply {
        status,
        headers,
        body: raw[split + 4..].to_vec(),
    };
    if reply
        .header("transfer-encoding")
        .is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
    {
        reply.body = unchunk(&reply.body);
    }
    Ok(reply)
}

/// The body of a chunked response, which must end with its last chunk.
fn unchunk(mut raw: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    loop {
        let line = raw.windows(2).position(|w| w == b"\r\n").unwrap();
        let size = std::str::from_utf8(&raw[..line]).unwrap();
        let size = usize::from_str_radix(size.split(';').next().unwrap().trim(), 16).unwrap();
        raw = &raw[line + 2..];
        if size == 0 {
            return body;
        }
        body.extend_from_slice(&raw[..size]);
        raw = &raw[size + 2..];
    }
}

/// Claim the instance as nova and sign in; returns the session cookie.
fn claim_and_sign_in(server: &Server) -> String {
    let credentials = br#"{"username":"nova","password":"correct-horse-battery"}"#;
    let json = [("Content-Type", "application/json")];
    let claim = send(server, "POST", "/api/claim", &json, credentials).unwrap();
    assert_eq!(claim.status, 201);
    let session = send(server, "POST", "/api/session", &json, credentials).unwrap();
    assert_eq!(session.status, 200);
    let cookie = session.header("set-cookie").unwrap();
    cookie.split(';').next().unwrap().to_owned()
}

fn upload(server: &Server, cookie: &str, name: &str, content_type: &str, bytes: &[u8]) -> Value {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: {content_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    let content_type = format!("multipart/form-data; boundary={BOUNDARY}");
    let reply = send(
        server,
        "POST",
        "/api/assets",
        &[("Cookie", cookie), ("Content-Type", &content_type)],
        &body,
    )
    .unwrap();
    assert_eq!(
        reply.status,
        201,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    reply.json()["asset"].clone()
}

fn list(server: &Server, cookie: &str) -> Value {
    let reply = send(server, "GET", "/api/assets", &[("Cookie", cookie)], b"").unwrap();
    assert_eq!(reply.status, 200);
    reply.json()
}

/// The original's media type and bytes.
fn original(server: &Server, cookie: &str, entry: &Value) -> (String, Vec<u8>) {
    let url = entry["original_url"].as_str().unwrap();
    let reply = send(server, "GET", url, &[("Cookie", cookie)], b"").unwrap();
    assert_eq!(reply.status, 200);
    (reply.header("content-type").unwrap().to_owned(), reply.body)
}

/// Every original with its current version's record, ordered by name, each
/// value as text: id, kind, name, version id, hash, size, media type, Source
/// id, Source version and ingestion time.
async fn records(db: &TestDatabase) -> Vec<Vec<String>> {
    rows(
        db,
        "SELECT a.id::text, a.kind, a.name, a.current_version_id::text, v.content_hash,
                v.byte_size::text, v.media_type, v.source_id, v.source_version,
                v.ingested_at::text
         FROM public.assets a
         JOIN public.asset_versions v ON v.asset_id = a.id AND v.id = a.current_version_id
         WHERE a.origin = 'source'
         ORDER BY a.name",
    )
    .await
}

/// Every preview with its lineage row, ordered by its original's name, each
/// value as text: the original's name, the preview's Asset id, version id,
/// hash, media type and producer, and the lineage row's input version, role
/// and run.
async fn previews(db: &TestDatabase) -> Vec<Vec<String>> {
    rows(
        db,
        "SELECT o.name, p.id::text, v.id::text, v.content_hash, v.media_type, v.source_id,
                l.input_version_id::text, l.role, l.run_id::text
         FROM public.asset_lineage l
         JOIN public.asset_versions v ON v.id = l.output_version_id
         JOIN public.assets p ON p.id = v.asset_id
         JOIN public.asset_versions i ON i.id = l.input_version_id
         JOIN public.assets o ON o.id = i.asset_id
         ORDER BY o.name",
    )
    .await
}

async fn rows(db: &TestDatabase, sql: &'static str) -> Vec<Vec<String>> {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let rows = sqlx::query(sql).fetch_all(&mut conn).await.unwrap();
    conn.close().await.unwrap();
    rows.iter()
        .map(|row| (0..row.len()).map(|i| row.get::<String, _>(i)).collect())
        .collect()
}

async fn stored_id(db: &TestDatabase) -> String {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let id: String = sqlx::query_scalar("SELECT storage_id::text FROM public.sidereal_metadata")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
    id
}

#[tokio::test]
async fn journey_keeps_a_png_and_a_jpeg_across_a_restart() {
    let db = TestDatabase::new();
    let storage = Dir::new("restart");

    let first = start(&db, &storage.0);
    let cookie = claim_and_sign_in(&first);
    let png = upload(&first, &cookie, "nebula.png", "image/png", PNG);
    // A JPEG named .png is still a JPEG: the bytes decide.
    let jpeg = upload(&first, &cookie, "m31.png", "image/png", JPEG);
    let listed = list(&first, &cookie);
    let before = records(&db).await;
    assert_eq!(before.len(), 2);
    let kept_previews = previews(&db).await;
    assert_eq!(kept_previews.len(), 2);
    for (preview, record) in kept_previews.iter().zip(&before) {
        assert_eq!(preview[0], record[2]);
        assert_eq!(preview[4], "image/jpeg");
        assert_eq!(preview[5], "image.make-preview");
        assert_eq!(preview[6], record[3]);
        assert_eq!(preview[7], "preview");
    }
    for entry in [&png, &jpeg] {
        let id = entry["id"].as_str().unwrap();
        assert_eq!(entry["preview_url"], format!("/api/assets/{id}/preview"));
    }
    for (record, (media, bytes)) in before
        .iter()
        .zip([("image/jpeg", JPEG), ("image/png", PNG)])
    {
        assert_eq!(record[1], "image");
        assert_eq!(record[5], bytes.len().to_string());
        assert_eq!(record[6], media);
        assert_eq!(record[7], "image.manual-upload");
    }
    // Opaque ids: neither the filename nor the content hash.
    for record in &before {
        assert_eq!(record[4].len(), 64);
        for id in [&record[0], &record[3]] {
            assert!(uuid::Uuid::parse_str(id).is_ok(), "{id}");
            assert!(!record[4].contains(id.replace('-', "").as_str()));
        }
    }
    assert_eq!(
        original(&first, &cookie, &png),
        ("image/png".to_owned(), PNG.to_vec())
    );
    assert_eq!(
        original(&first, &cookie, &jpeg),
        ("image/jpeg".to_owned(), JPEG.to_vec())
    );
    drop(first);

    let second = start(&db, &storage.0);
    assert_eq!(list(&second, &cookie), listed);
    assert_eq!(records(&db).await, before);
    assert_eq!(previews(&db).await, kept_previews);
    assert_eq!(
        original(&second, &cookie, &png),
        ("image/png".to_owned(), PNG.to_vec())
    );
    assert_eq!(
        original(&second, &cookie, &jpeg),
        ("image/jpeg".to_owned(), JPEG.to_vec())
    );
}

#[tokio::test]
async fn journey_keeps_an_original_when_storage_root_is_a_link() {
    let db = TestDatabase::new();
    let place = Dir::new("link");
    let target = place.0.join("target");
    std::fs::create_dir(&target).unwrap();
    let link = place.0.join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let first = start(&db, &link);
    let paired = stored_id(&db).await;
    let cookie = claim_and_sign_in(&first);
    let png = upload(&first, &cookie, "nebula.png", "image/png", PNG);
    drop(first);

    // The marker and the object live under the link's target.
    let marker = std::fs::read_to_string(target.join(MARKER)).unwrap();
    assert!(marker.contains(&paired));
    assert!(target.join("store").is_dir());
    assert!(std::fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());

    let second = start(&db, &link);
    assert_eq!(stored_id(&db).await, paired);
    assert_eq!(
        std::fs::read_to_string(target.join(MARKER)).unwrap(),
        marker
    );
    assert_eq!(
        original(&second, &cookie, &png),
        ("image/png".to_owned(), PNG.to_vec())
    );
}
