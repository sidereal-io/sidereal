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
