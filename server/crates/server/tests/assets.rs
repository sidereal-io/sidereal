//! The Asset routes, in process, against a fresh database and a fresh paired
//! storage folder per test, with the production Registry.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use serde_json::Value;
use sidereal_core::{
    database::{initialize, initialize_with_storage},
    storage::{objects::ObjectStore, StorageRoot},
};
use sidereal_server::{
    app_with_database, app_with_library, config::AllowedOrigins, Library, Tuning,
};
use sqlx::{Connection, PgConnection};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PASSWORD: &str = "correct-horse-battery";
const SECONDS: Duration = Duration::from_secs(10);
const PNG: &[u8] = include_bytes!("fixtures/nebula.png");
const JPEG: &[u8] = include_bytes!("fixtures/nebula.jpg");
const LIMIT: usize = 104_857_600;
const BOUNDARY: &str = "sidereal-test-boundary";

/// A throwaway storage folder under the target tree.
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "assets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
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

/// Every file under `path`, sorted, with its bytes.
fn files(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(path: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), out);
            } else {
                out.push((entry.path(), std::fs::read(entry.path()).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(path, &mut out);
    out.sort();
    out
}

/// A signed-in administrator on a server with paired storage.
struct Instance {
    db: TestDatabase,
    storage: Option<Dir>,
    app: Router,
    cookie: String,
}

impl Instance {
    async fn new() -> Self {
        let db = TestDatabase::new();
        let storage = Dir::new();
        let root = StorageRoot::new(&storage.0).unwrap();
        let (database, paired) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root)
                .await
                .unwrap();
        let objects = ObjectStore::open(&root, paired).unwrap();
        let app = app_with_library(
            database.clone(),
            AllowedOrigins::default(),
            Tuning::default(),
            Library::new(Some(objects)),
        );
        Self::signed_in(db, Some(storage), app).await
    }

    /// A server started without `STORAGE_ROOT`.
    async fn without_storage() -> Self {
        let db = TestDatabase::new();
        let database = initialize(db.options.clone(), SECONDS, SECONDS)
            .await
            .unwrap();
        let app = app_with_database(database.clone(), AllowedOrigins::default());
        Self::signed_in(db, None, app).await
    }

    async fn signed_in(db: TestDatabase, storage: Option<Dir>, app: Router) -> Self {
        let credentials =
            serde_json::json!({ "username": "sam", "password": PASSWORD }).to_string();
        for uri in ["/api/claim", "/api/session"] {
            let response = app
                .clone()
                .oneshot(
                    Request::post(uri)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(credentials.clone()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert!(
                response.status().is_success(),
                "{uri}: {}",
                response.status()
            );
            if uri == "/api/session" {
                let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
                let cookie = cookie.split(';').next().unwrap().to_owned();
                return Self {
                    db,
                    storage,
                    app,
                    cookie,
                };
            }
        }
        unreachable!()
    }

    fn storage(&self) -> &Path {
        &self.storage.as_ref().unwrap().0
    }

    async fn send(&self, request: Request<Body>) -> Response {
        self.app.clone().oneshot(request).await.unwrap()
    }

    fn upload_request(&self) -> axum::http::request::Builder {
        Request::post("/api/assets")
            .header(header::COOKIE, &self.cookie)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
    }

    async fn upload_body(&self, body: Vec<u8>) -> Response {
        let request = self.upload_request();
        self.send(request.body(Body::from(body)).unwrap()).await
    }

    async fn upload(&self, name: &str, bytes: &[u8]) -> Response {
        self.upload_body(multipart(
            &[Part::file("file", name, "image/png", bytes)],
            true,
            b"",
        ))
        .await
    }

    async fn rows(&self) -> i64 {
        let mut conn = PgConnection::connect_with(&self.db.options).await.unwrap();
        sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM public.assets) + (SELECT count(*) FROM public.asset_versions)",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap()
    }

    fn store_files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        files(&self.storage().join("store"))
    }
}

struct Part<'a> {
    name: &'a str,
    filename: Option<&'a str>,
    content_type: &'a str,
    bytes: &'a [u8],
}

impl<'a> Part<'a> {
    fn file(name: &'a str, filename: &'a str, content_type: &'a str, bytes: &'a [u8]) -> Self {
        Self {
            name,
            filename: Some(filename),
            content_type,
            bytes,
        }
    }
}

/// A body whose chunks arrive one at a time, as from a network: each chunk
/// is ready only after the reader has once found nothing to read.
fn arriving<T: Send + 'static>(
    chunks: impl IntoIterator<Item = T>,
) -> impl futures_util::Stream<Item = T> + Send + 'static {
    use futures_util::StreamExt;
    let chunks: Vec<T> = chunks.into_iter().collect();
    futures_util::stream::iter(chunks).then(|chunk| async move {
        tokio::task::yield_now().await;
        chunk
    })
}

/// A multipart body. Without `closing`, it ends after the last part's bytes.
fn multipart(parts: &[Part], closing: bool, epilogue: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        let disposition = match part.filename {
            Some(filename) => format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{filename}\"\r\n",
                part.name
            ),
            None => format!("Content-Disposition: form-data; name=\"{}\"\r\n", part.name),
        };
        body.extend_from_slice(disposition.as_bytes());
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", part.content_type).as_bytes());
        body.extend_from_slice(part.bytes);
        body.extend_from_slice(b"\r\n");
    }
    if closing {
        body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
        body.extend_from_slice(epilogue);
    }
    body
}

async fn json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// `PNG` padded with zero bytes to `size`; its signature still says PNG.
fn png_of_size(size: usize) -> Vec<u8> {
    let mut bytes = PNG.to_vec();
    bytes.resize(size, 0);
    bytes
}

mod upload {
    use super::*;

    #[tokio::test]
    async fn upload_of_one_png_answers_201_with_its_entry() {
        let instance = Instance::new().await;
        let response = instance.upload("nebula.png", PNG).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let asset = &json(response).await["asset"];
        let id = asset["id"].as_str().unwrap();
        assert_eq!(asset["kind"], "image");
        assert_eq!(asset["name"], "nebula.png");
        assert!(asset["current_version_id"].as_str().is_some());
        assert_eq!(asset["original_url"], format!("/api/assets/{id}/original"));
        assert_eq!(instance.rows().await, 2);
    }

    #[tokio::test]
    async fn upload_publishes_the_original_bytes() {
        let instance = Instance::new().await;
        instance.upload("nebula.png", PNG).await;
        let stored = instance.store_files();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].1, PNG);
    }

    #[tokio::test]
    async fn upload_accepts_png_bytes_named_txt_with_a_text_plain_type() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file("file", "nebula.txt", "text/plain", PNG)],
            true,
            b"",
        );
        let response = instance.upload_body(body).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(json(response).await["asset"]["name"], "nebula.txt");
        let format: String = sqlx::query_scalar("SELECT media_type FROM public.asset_versions")
            .fetch_one(
                &mut PgConnection::connect_with(&instance.db.options)
                    .await
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(format, "image/png");
    }

    #[tokio::test]
    async fn upload_of_a_jpeg_records_its_format() {
        let instance = Instance::new().await;
        let response = instance.upload("nebula.jpg", JPEG).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let format: String = sqlx::query_scalar("SELECT media_type FROM public.asset_versions")
            .fetch_one(
                &mut PgConnection::connect_with(&instance.db.options)
                    .await
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(format, "image/jpeg");
    }

    #[tokio::test]
    async fn upload_of_exactly_100_mib_is_accepted() {
        let instance = Instance::new().await;
        let response = instance.upload("big.png", &png_of_size(LIMIT)).await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn upload_one_byte_over_100_mib_streamed_without_a_length_answers_413() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file(
                "file",
                "big.png",
                "image/png",
                &png_of_size(LIMIT + 1),
            )],
            true,
            b"",
        );
        // A stream of chunks has no Content-Length.
        let chunks: Vec<Result<Vec<u8>, std::io::Error>> =
            body.chunks(64 * 1024).map(|c| Ok(c.to_vec())).collect();
        let request = instance
            .upload_request()
            .body(Body::from_stream(futures_util::stream::iter(chunks)))
            .unwrap();
        assert!(request.headers().get(header::CONTENT_LENGTH).is_none());
        let response = instance.send(request).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert!(json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("100 MiB"));
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    #[tokio::test]
    async fn upload_with_two_file_fields_answers_400_and_commits_nothing() {
        let instance = Instance::new().await;
        let body = multipart(
            &[
                Part::file("file", "a.png", "image/png", PNG),
                Part::file("file", "b.jpg", "image/jpeg", JPEG),
            ],
            true,
            b"",
        );
        assert_eq!(
            instance.upload_body(body).await.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    #[tokio::test]
    async fn upload_with_an_extra_field_answers_400() {
        let instance = Instance::new().await;
        let body = multipart(
            &[
                Part::file("file", "a.png", "image/png", PNG),
                Part {
                    name: "note",
                    filename: None,
                    content_type: "text/plain",
                    bytes: b"hello",
                },
            ],
            true,
            b"",
        );
        assert_eq!(
            instance.upload_body(body).await.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(instance.rows().await, 0);
    }

    #[tokio::test]
    async fn upload_with_no_field_named_file_answers_400() {
        let instance = Instance::new().await;
        let body = multipart(&[Part::file("image", "a.png", "image/png", PNG)], true, b"");
        assert_eq!(
            instance.upload_body(body).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn upload_with_an_empty_filename_answers_400() {
        let instance = Instance::new().await;
        let body = multipart(&[Part::file("file", "", "image/png", PNG)], true, b"");
        assert_eq!(
            instance.upload_body(body).await.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(instance.rows().await, 0);
    }

    #[tokio::test]
    async fn upload_with_no_parts_answers_400() {
        let instance = Instance::new().await;
        assert_eq!(
            instance
                .upload_body(multipart(&[], true, b""))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn upload_cut_off_before_the_closing_delimiter_answers_400_and_commits_nothing() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            false,
            b"",
        );
        assert_eq!(
            instance.upload_body(body).await.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    #[tokio::test]
    async fn upload_ignores_an_epilogue_after_the_closing_delimiter() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            true,
            b"an epilogue the server ignores",
        );
        assert_eq!(
            instance.upload_body(body).await.status(),
            StatusCode::CREATED
        );
        assert_eq!(instance.store_files()[0].1, PNG);
    }

    #[tokio::test]
    async fn upload_whose_epilogue_passes_the_transport_limit_answers_413() {
        let instance = Instance::new().await;
        let epilogue = vec![b'x'; 105_906_176];
        let body = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            true,
            &epilogue,
        );
        let request = instance
            .upload_request()
            .header(header::CONTENT_LENGTH, body.len())
            .body(Body::from(body))
            .unwrap();
        assert_eq!(
            instance.send(request).await.status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    #[tokio::test]
    async fn streamed_upload_whose_epilogue_passes_the_transport_limit_answers_413() {
        let instance = Instance::new().await;
        let head = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            true,
            b"",
        );
        // 102 chunks of 1 MiB after the closing delimiter, with no
        // Content-Length, so only reading the epilogue finds the excess.
        let filler = axum::body::Bytes::from(vec![b'x'; 1024 * 1024]);
        let chunks = std::iter::once(axum::body::Bytes::from(head))
            .chain(std::iter::repeat_n(filler, 102))
            .map(Ok::<_, std::io::Error>);
        let request = instance
            .upload_request()
            .body(Body::from_stream(arriving(chunks)))
            .unwrap();
        assert!(request.headers().get(header::CONTENT_LENGTH).is_none());
        assert_eq!(
            instance.send(request).await.status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    #[tokio::test]
    async fn streamed_upload_with_a_small_epilogue_answers_201() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            true,
            b"",
        );
        let chunks = [
            axum::body::Bytes::from(body),
            axum::body::Bytes::from_static(b"an epilogue in its own chunk"),
        ]
        .map(Ok::<_, std::io::Error>);
        let request = instance
            .upload_request()
            .body(Body::from_stream(arriving(chunks)))
            .unwrap();
        assert_eq!(instance.send(request).await.status(), StatusCode::CREATED);
        assert_eq!(instance.store_files()[0].1, PNG);
    }

    #[tokio::test]
    async fn upload_of_unsupported_bytes_answers_415_naming_png_and_jpeg() {
        let instance = Instance::new().await;
        for (name, bytes) in [
            ("nebula.tif", &b"II*\x00\x08\x00\x00\x00"[..]),
            ("nebula.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            ("empty.png", b""),
            ("nebula.jpg", b"plain text, not a JPEG"),
        ] {
            let response = instance.upload(name, bytes).await;
            assert_eq!(
                response.status(),
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "{name}"
            );
            let error = json(response).await["error"].as_str().unwrap().to_owned();
            assert!(error.contains("PNG") && error.contains("JPEG"), "{error}");
        }
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    #[tokio::test]
    async fn upload_of_the_same_bytes_again_answers_409_with_the_existing_entry() {
        let instance = Instance::new().await;
        let response = instance.upload("nebula.png", PNG).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let first = json(response).await["asset"].clone();
        for name in ["nebula.png", "m31-final.png"] {
            let response = instance.upload(name, PNG).await;
            assert_eq!(response.status(), StatusCode::CONFLICT, "{name}");
            let found = json(response).await;
            assert_eq!(found, serde_json::json!({ "asset": first }), "{name}");
            assert_eq!(found["asset"]["name"], "nebula.png");
        }
        assert_eq!(
            json(instance.list().await).await["assets"],
            serde_json::json!([first])
        );
        assert_eq!(instance.rows().await, 2);
    }

    #[tokio::test]
    async fn plain_text_named_like_an_existing_image_still_answers_415() {
        let instance = Instance::new().await;
        assert_eq!(
            instance.upload("nebula.png", PNG).await.status(),
            StatusCode::CREATED
        );
        let response = instance
            .upload("nebula.png", b"plain text, not a PNG")
            .await;
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(instance.rows().await, 2);
    }

    #[tokio::test]
    async fn upload_onto_an_object_whose_bytes_changed_answers_503() {
        let instance = Instance::new().await;
        instance.upload("first.png", PNG).await;
        let (path, _) = instance.store_files().remove(0);
        let damaged = vec![0u8; PNG.len()];
        std::fs::write(&path, &damaged).unwrap();
        let response = instance.upload("second.png", PNG).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(instance.rows().await, 2);
        assert_eq!(std::fs::read(&path).unwrap(), damaged);
    }

    #[tokio::test]
    async fn upload_onto_a_much_larger_object_answers_503_and_leaves_it_unchanged() {
        let instance = Instance::new().await;
        instance.upload("first.png", PNG).await;
        let (path, _) = instance.store_files().remove(0);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(1 << 40)
            .unwrap();
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let response = instance.upload("second.png", PNG).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(instance.rows().await, 2);
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(
            (metadata.len(), metadata.modified().unwrap()),
            (1 << 40, modified)
        );
    }

    #[tokio::test]
    async fn upload_without_storage_answers_503_and_leaves_health_and_readiness_alone() {
        let instance = Instance::without_storage().await;
        let response = instance.upload("nebula.png", PNG).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("storage"));
        assert_eq!(instance.rows().await, 0);
        for (uri, body) in [
            ("/healthz", r#"{"status":"ok"}"#),
            ("/readyz", r#"{"status":"ready"}"#),
        ] {
            let response = instance
                .send(Request::get(uri).body(Body::empty()).unwrap())
                .await;
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            let bytes = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert_eq!(bytes, body.as_bytes(), "{uri}");
        }
    }

    #[tokio::test]
    async fn upload_without_a_session_answers_401_and_stores_nothing() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            true,
            b"",
        );
        let before = files(instance.storage());
        for cookie in [None, Some("sidereal_session=not-a-session")] {
            let mut request = Request::post("/api/assets").header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            );
            if let Some(cookie) = cookie {
                request = request.header(header::COOKIE, cookie);
            }
            let response = instance
                .send(request.body(Body::from(body.clone())).unwrap())
                .await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        assert_eq!(instance.rows().await, 0);
        assert_eq!(files(instance.storage()), before);
        assert!(!instance.storage().join("tmp").exists());
    }

    #[tokio::test]
    async fn upload_from_a_forbidden_origin_answers_403_and_stores_nothing() {
        let instance = Instance::new().await;
        let body = multipart(
            &[Part::file("file", "nebula.png", "image/png", PNG)],
            true,
            b"",
        );
        let before = files(instance.storage());
        let request = instance
            .upload_request()
            .header(header::HOST, "photos.example")
            .header(header::ORIGIN, "https://elsewhere.example")
            .body(Body::from(body))
            .unwrap();
        assert_eq!(instance.send(request).await.status(), StatusCode::FORBIDDEN);
        assert_eq!(instance.rows().await, 0);
        assert_eq!(files(instance.storage()), before);
        assert!(!instance.storage().join("tmp").exists());
    }
}

impl Instance {
    async fn get(&self, uri: &str, cookie: Option<&str>) -> Response {
        let mut request = Request::get(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }

    async fn list(&self) -> Response {
        self.get("/api/assets", Some(&self.cookie)).await
    }

    async fn execute(&self, sql: &str) {
        let mut conn = PgConnection::connect_with(&self.db.options).await.unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql.to_owned()))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    /// The administrator's cookie, after `sql` changed its session.
    async fn with_session(&self, sql: &str) -> &str {
        self.execute(sql).await;
        &self.cookie
    }
}

async fn body_text(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Ways a request can lack a valid session: none, expired, revoked.
const INVALID_SESSIONS: [(&str, Option<&str>); 3] = [
    ("absent", None),
    (
        "expired",
        Some("UPDATE public.sessions SET expires_at = now() - interval '1 second'"),
    ),
    ("revoked", Some("DELETE FROM public.sessions")),
];

mod list {
    use super::*;

    #[tokio::test]
    async fn list_of_an_empty_library_is_an_empty_array() {
        let instance = Instance::new().await;
        let response = instance.list().await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_text(response).await, r#"{"assets":[]}"#);
    }

    #[tokio::test]
    async fn list_after_upload_shows_the_committed_entry() {
        let instance = Instance::new().await;
        let uploaded = json(instance.upload("nebula.jpg", JPEG).await).await["asset"].clone();
        let response = instance.list().await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json(response).await["assets"],
            serde_json::json!([uploaded])
        );
    }

    #[tokio::test]
    async fn list_puts_older_uploads_first() {
        let instance = Instance::new().await;
        instance.upload("first.png", PNG).await;
        instance.upload("second.jpg", JPEG).await;
        let names: Vec<_> = json(instance.list().await).await["assets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["name"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["first.png", "second.jpg"]);
    }

    #[tokio::test]
    async fn list_ignores_staging_files_and_unreferenced_objects() {
        let instance = Instance::new().await;
        instance.upload("nebula.png", PNG).await;
        let stray = instance
            .storage()
            .join("tmp/00000000-0000-4000-8000-000000000000");
        std::fs::write(&stray, b"left by a crash").unwrap();
        let hash = "0".repeat(64);
        let orphan = instance.storage().join(format!("store/00/00/{hash}"));
        std::fs::create_dir_all(orphan.parent().unwrap()).unwrap();
        std::fs::write(&orphan, b"published, never committed").unwrap();
        let assets = json(instance.list().await).await["assets"].clone();
        assert_eq!(assets.as_array().unwrap().len(), 1);
        assert_eq!(assets[0]["name"], "nebula.png");
    }

    #[tokio::test]
    async fn list_answers_503_when_the_database_fails_after_the_session_check() {
        let instance = Instance::new().await;
        instance
            .execute("ALTER TABLE public.assets RENAME TO assets_moved")
            .await;
        let response = instance.list().await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(!body_text(response).await.contains("assets"));
    }

    #[tokio::test]
    async fn list_without_a_valid_session_answers_401_with_no_metadata() {
        for (case, sql) in INVALID_SESSIONS {
            let instance = Instance::new().await;
            instance.upload("nebula.png", PNG).await;
            let cookie = match sql {
                Some(sql) => Some(instance.with_session(sql).await),
                None => None,
            };
            let response = instance.get("/api/assets", cookie).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{case}");
            let body = body_text(response).await;
            assert!(!body.contains("nebula"), "{case}: {body}");
        }
    }
}

mod original {
    use super::*;
    use std::{
        io::Read,
        os::unix::fs::{symlink, PermissionsExt},
        sync::Arc,
        time::Instant,
    };

    impl Instance {
        /// Upload `bytes` and answer the new Asset's id and object path.
        async fn uploaded(&self, name: &str, bytes: &[u8]) -> (String, PathBuf) {
            let response = self.upload(name, bytes).await;
            assert_eq!(response.status(), StatusCode::CREATED);
            let id = json(response).await["asset"]["id"]
                .as_str()
                .unwrap()
                .to_owned();
            let path = self
                .store_files()
                .into_iter()
                .find(|(_, stored)| stored == bytes)
                .unwrap()
                .0;
            (id, path)
        }

        async fn original(&self, id: &str) -> Response {
            self.get(&format!("/api/assets/{id}/original"), Some(&self.cookie))
                .await
        }

        async fn library_rows(&self) -> String {
            let mut conn = PgConnection::connect_with(&self.db.options).await.unwrap();
            sqlx::query_scalar(
                "SELECT (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM public.assets a)::text \
                 || (SELECT coalesce(jsonb_agg(to_jsonb(v) ORDER BY id), '[]') FROM public.asset_versions v)::text",
            )
            .fetch_one(&mut conn)
            .await
            .unwrap()
        }
    }

    async fn bytes(response: Response) -> Vec<u8> {
        axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap()
            .to_vec()
    }

    /// Answer 503 with no file bytes, and change no library row.
    async fn assert_unavailable(instance: &Instance, id: &str) {
        let before = instance.library_rows().await;
        let start = Instant::now();
        let response = instance.original(id).await;
        assert!(start.elapsed() < Duration::from_secs(5));
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = bytes(response).await;
        assert!(
            !body.windows(4).any(|w| w == &PNG[..4]),
            "file bytes in the body"
        );
        assert_eq!(instance.library_rows().await, before);
    }

    #[tokio::test]
    async fn original_of_a_png_returns_its_exact_bytes_as_image_png() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded("nebula.png", PNG).await;
        let response = instance.original(&id).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
        assert_eq!(bytes(response).await, PNG);
    }

    #[tokio::test]
    async fn original_responses_are_private_and_never_sniffed() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded("nebula.png", PNG).await;
        let response = instance.original(&id).await;
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }

    #[tokio::test]
    async fn original_of_a_jpeg_named_png_is_image_jpeg() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded("nebula.png", JPEG).await;
        let response = instance.original(&id).await;
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(bytes(response).await, JPEG);
    }

    #[tokio::test]
    async fn original_of_an_unknown_id_answers_404_with_no_bytes() {
        let instance = Instance::new().await;
        instance.uploaded("nebula.png", PNG).await;
        for id in ["00000000-0000-4000-8000-000000000000", "not-an-id"] {
            let response = instance.original(id).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{id}");
            let body = bytes(response).await;
            assert!(!body.windows(4).any(|w| w == &PNG[..4]), "{id}");
        }
    }

    #[tokio::test]
    async fn original_of_a_missing_object_answers_503() {
        let instance = Instance::new().await;
        let (id, path) = instance.uploaded("nebula.png", PNG).await;
        std::fs::remove_file(path).unwrap();
        assert_unavailable(&instance, &id).await;
    }

    #[tokio::test]
    async fn original_of_an_unopenable_object_answers_503() {
        // SAFETY: geteuid has no preconditions and cannot fail.
        if unsafe { libc::geteuid() } == 0 {
            return; // root reads files whatever their mode
        }
        let instance = Instance::new().await;
        let (id, path) = instance.uploaded("nebula.png", PNG).await;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        assert_unavailable(&instance, &id).await;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[tokio::test]
    async fn original_of_a_folder_at_the_object_path_answers_503() {
        let instance = Instance::new().await;
        let (id, path) = instance.uploaded("nebula.png", PNG).await;
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert_unavailable(&instance, &id).await;
    }

    #[tokio::test]
    async fn original_of_a_fifo_with_no_writer_answers_503_promptly() {
        let instance = Instance::new().await;
        let (id, path) = instance.uploaded("nebula.png", PNG).await;
        std::fs::remove_file(&path).unwrap();
        let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: `c` is a NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
        assert_unavailable(&instance, &id).await;
    }

    #[tokio::test]
    async fn original_through_a_symlinked_object_answers_503() {
        let instance = Instance::new().await;
        let (id, path) = instance.uploaded("nebula.png", PNG).await;
        let outside = Dir::new();
        std::fs::write(outside.0.join("copy"), PNG).unwrap();
        std::fs::remove_file(&path).unwrap();
        symlink(outside.0.join("copy"), &path).unwrap();
        assert_unavailable(&instance, &id).await;
    }

    #[tokio::test]
    async fn original_through_a_symlinked_prefix_folder_answers_503() {
        let instance = Instance::new().await;
        let (id, path) = instance.uploaded("nebula.png", PNG).await;
        let prefix = path.parent().unwrap().to_owned();
        let outside = Dir::new();
        std::fs::rename(&prefix, outside.0.join("moved")).unwrap();
        symlink(outside.0.join("moved"), &prefix).unwrap();
        assert_unavailable(&instance, &id).await;
    }

    #[tokio::test]
    async fn original_of_an_unknown_asset_without_storage_answers_404() {
        let instance = Instance::without_storage().await;
        let response = instance
            .original("00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn original_of_an_asset_on_a_server_without_storage_answers_503() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded("nebula.png", PNG).await;
        // The same database and session, served by a server started
        // without STORAGE_ROOT.
        let database =
            sidereal_core::database::Database::open(instance.db.options.clone(), SECONDS)
                .await
                .unwrap();
        let app = app_with_database(database, AllowedOrigins::default());
        let response = app
            .oneshot(
                Request::get(format!("/api/assets/{id}/original"))
                    .header(header::COOKIE, &instance.cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    /// A reader that gives its first bytes, then fails.
    struct FailsAfter(Vec<u8>);
    impl Read for FailsAfter {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0.is_empty() {
                return Err(std::io::Error::other("injected read failure"));
            }
            let n = buf.len().min(self.0.len());
            buf[..n].copy_from_slice(&self.0[..n]);
            self.0.drain(..n);
            Ok(n)
        }
    }

    #[tokio::test]
    async fn original_whose_read_fails_midway_is_a_failed_transfer() {
        let db = TestDatabase::new();
        let storage = Dir::new();
        let root = StorageRoot::new(&storage.0).unwrap();
        let (database, paired) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root)
                .await
                .unwrap();
        let objects = ObjectStore::open(&root, paired).unwrap();
        let library = Library::new(Some(objects)).with_original_reader(Arc::new(|_file| {
            Box::new(FailsAfter(PNG[..20].to_vec())) as Box<dyn Read + Send>
        }));
        let app = app_with_library(
            database,
            AllowedOrigins::default(),
            Tuning::default(),
            library,
        );
        let instance = Instance::signed_in(db, Some(storage), app).await;
        let (id, _) = instance.uploaded("nebula.png", PNG).await;
        let before = instance.library_rows().await;
        let response = instance.original(&id).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .is_err());
        assert_eq!(instance.library_rows().await, before);
    }

    #[tokio::test]
    async fn original_without_a_valid_session_answers_the_guards_401() {
        for (case, sql) in INVALID_SESSIONS {
            let instance = Instance::new().await;
            let (id, _) = instance.uploaded("nebula.png", PNG).await;
            let cookie = match sql {
                Some(sql) => Some(instance.with_session(sql).await.to_owned()),
                None => None,
            };
            let original = instance
                .get(&format!("/api/assets/{id}/original"), cookie.as_deref())
                .await;
            let session = instance.get("/api/session", cookie.as_deref()).await;
            assert_eq!(original.status(), StatusCode::UNAUTHORIZED, "{case}");
            let names = |response: &Response| {
                let mut names: Vec<_> = response.headers().keys().map(|k| k.to_string()).collect();
                names.sort();
                names
            };
            assert_eq!(names(&original), names(&session), "{case}");
            assert!(original.headers().get("x-content-type-options").is_none());
            assert_eq!(bytes(original).await, bytes(session).await, "{case}");
        }
    }
}

mod web_fallback {
    use super::*;
    use sidereal_server::{config::web_root, with_web_root};

    const INDEX: &str = "<!doctype html><title>first build</title>";

    /// A signed-in instance whose storage is `storage` and whose web shell
    /// is served from `web`, resolved as startup resolves `WEB_ROOT`.
    async fn serving(storage: &Path, web: &Path) -> Instance {
        let db = TestDatabase::new();
        let root = StorageRoot::new(storage).unwrap();
        let web = web_root(Some(web.as_os_str())).unwrap().unwrap();
        sidereal_server::config::check_overlap(&root, &web).unwrap();
        let (database, paired) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root)
                .await
                .unwrap();
        let objects = ObjectStore::open(&root, paired).unwrap();
        let app = with_web_root(
            app_with_library(
                database,
                AllowedOrigins::default(),
                Tuning::default(),
                Library::new(Some(objects)),
            ),
            Some(&web),
        );
        Instance::signed_in(db, None, app).await
    }

    fn holds_png(body: &[u8]) -> bool {
        body.windows(PNG.len()).any(|w| w == PNG)
    }

    #[tokio::test]
    async fn web_fallback_serves_no_stored_object_without_a_session() {
        let parent = Dir::new();
        let (web, storage) = (parent.0.join("web"), parent.0.join("storage"));
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("index.html"), INDEX).unwrap();
        std::fs::create_dir_all(&storage).unwrap();
        let instance = serving(&storage, &web).await;
        assert_eq!(
            instance.upload("nebula.png", PNG).await.status(),
            StatusCode::CREATED
        );
        let (object, _) = files(&storage.join("store")).remove(0);
        let relative = object
            .strip_prefix(&storage)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        for uri in [format!("/{relative}"), format!("/../storage/{relative}")] {
            let response = instance.get(&uri, None).await;
            assert_ne!(response.status(), StatusCode::OK, "{uri}");
            let body = axum::body::to_bytes(response.into_body(), 1 << 20)
                .await
                .unwrap();
            assert!(!holds_png(&body), "{uri}");
        }
    }

    #[tokio::test]
    async fn web_fallback_keeps_serving_the_web_root_resolved_at_startup() {
        let parent = Dir::new();
        let (build, data) = (parent.0.join("build"), parent.0.join("data"));
        let storage = data.join("storage");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("index.html"), INDEX).unwrap();
        std::fs::create_dir_all(&storage).unwrap();
        let link = parent.0.join("current");
        std::os::unix::fs::symlink(&build, &link).unwrap();
        let instance = serving(&storage, &link).await;
        assert_eq!(
            instance.upload("nebula.png", PNG).await.status(),
            StatusCode::CREATED
        );

        // Repoint the link at the folder that holds the store.
        std::fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(&data, &link).unwrap();

        let (object, _) = files(&storage.join("store")).remove(0);
        let relative = object
            .strip_prefix(&data)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let response = instance.get(&format!("/{relative}"), None).await;
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        assert!(!holds_png(&body));
        let home = instance.get("/", None).await;
        assert_eq!(home.status(), StatusCode::OK);
        assert_eq!(body_text(home).await, INDEX);
    }
}
