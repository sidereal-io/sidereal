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
use sidereal_pack_image::{ImagePack, FILE_SIZE, MAKE_PREVIEW, READ_HEADER};
use sidereal_plugin_abi::{
    AssetContext, FacetSchema, FacetScope, FacetType, FacetValue, Operator, OperatorError, Pack,
    Registry,
};
use sidereal_server::preview::{WorkerCommand, WorkerRunner};
use sidereal_server::{
    app_with_database, app_with_library, config::AllowedOrigins, Library, Log, Tuning,
};
use sqlx::{Connection, PgConnection};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
#[path = "../../packs/image/tests/common/tiff.rs"]
mod tiff;
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
    /// Every line the server wrote to its operational log.
    lines: Arc<Mutex<Vec<String>>>,
}

impl Instance {
    async fn new() -> Self {
        Self::with_library(|library| library).await
    }

    /// A server whose production `Library` the test changes first.
    async fn with_library(change: impl FnOnce(Library) -> Library) -> Self {
        let db = TestDatabase::new();
        let storage = Dir::new();
        let root = StorageRoot::new(&storage.0).unwrap();
        let (database, paired, _) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root)
                .await
                .unwrap();
        let objects = ObjectStore::open(&root, paired).unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let written = lines.clone();
        let app = app_with_library(
            database.clone(),
            AllowedOrigins::default(),
            Tuning {
                log: Log::new(move |line| written.lock().unwrap().push(line.to_owned())),
                ..Tuning::default()
            },
            change(Library::new(Some(objects)).with_previews(helper())),
        );
        let mut instance = Self::signed_in(db, Some(storage), app).await;
        instance.lines = lines;
        instance
    }

    fn logged(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
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
                    lines: Arc::default(),
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

    /// Original Asset rows plus their version rows.
    async fn rows(&self) -> i64 {
        self.count(
            "SELECT (SELECT count(*) FROM public.assets WHERE origin = 'source') + \
             (SELECT count(*) FROM public.asset_versions v JOIN public.assets a \
              ON a.id = v.asset_id WHERE a.origin = 'source')",
        )
        .await
    }

    /// Derived Asset rows, their version rows and lineage rows.
    async fn derived_rows(&self) -> i64 {
        self.count(
            "SELECT (SELECT count(*) FROM public.assets WHERE origin = 'derived') + \
             (SELECT count(*) FROM public.asset_versions v JOIN public.assets a \
              ON a.id = v.asset_id WHERE a.origin = 'derived') + \
             (SELECT count(*) FROM public.asset_lineage)",
        )
        .await
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut conn = PgConnection::connect_with(&self.db.options).await.unwrap();
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    fn store_files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        files(&self.storage().join("store"))
    }

    /// The stored object that holds exactly `bytes`.
    fn object_holding(&self, bytes: &[u8]) -> PathBuf {
        let mut matching: Vec<_> = self
            .store_files()
            .into_iter()
            .filter(|(_, stored)| stored == bytes)
            .collect();
        assert_eq!(matching.len(), 1, "one stored object holds those bytes");
        matching.remove(0).0
    }

    /// The media type of the one original's version.
    async fn original_media_type(&self) -> String {
        let mut conn = PgConnection::connect_with(&self.db.options).await.unwrap();
        sqlx::query_scalar(
            "SELECT v.media_type FROM public.asset_versions v \
             JOIN public.assets a ON a.id = v.asset_id WHERE a.origin = 'source'",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap()
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

/// The three facets upload records for a 32 by 24 PNG or JPEG of `size`
/// bytes, as the API returns them.
fn upload_facets(size: usize) -> Value {
    serde_json::json!([
        {"name": "image.file-size", "label": "File size", "value": size, "unit": "bytes"},
        {"name": "image.height", "label": "Height", "value": 24, "unit": "pixels"},
        {"name": "image.width", "label": "Width", "value": 32, "unit": "pixels"},
    ])
}

impl Instance {
    /// Upload `bytes` as `name`, which must make a new Asset; its id and
    /// its version's id.
    async fn uploaded_ids(&self, name: &str, bytes: &[u8]) -> (String, String) {
        let response = self.upload(name, bytes).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let asset = &json(response).await["asset"];
        (
            asset["id"].as_str().unwrap().to_owned(),
            asset["current_version_id"].as_str().unwrap().to_owned(),
        )
    }

    /// The facets `GET /api/assets/{id}` returns.
    async fn facets_of(&self, id: &str) -> Value {
        let response = self
            .get(&format!("/api/assets/{id}"), Some(&self.cookie))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        json(response).await["asset"]["facets"].clone()
    }
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
        assert_eq!(asset["preview_url"], format!("/api/assets/{id}/preview"));
        assert_eq!(instance.rows().await, 2);
        // The preview: a derived Asset, its version and its lineage row.
        assert_eq!(instance.derived_rows().await, 3);
    }

    #[tokio::test]
    async fn upload_records_the_file_size_of_a_png_of_known_size() {
        let instance = Instance::new().await;
        let response = instance.upload("nebula.png", &png_of_size(1_234_567)).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let asset = &json(response).await["asset"];
        let version = asset["current_version_id"].as_str().unwrap();
        let mut conn = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        let (facet, value, on_version, on_asset, producer, producer_version): (
            String,
            Value,
            Option<String>,
            Option<String>,
            String,
            String,
        ) = sqlx::query_as(
            "SELECT facet, value, asset_version_id::text, asset_id::text, producer_id, producer_version \
             FROM public.facet_values WHERE facet = 'image.file-size'",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert_eq!(facet, "image.file-size");
        assert_eq!(value, serde_json::json!(1_234_567));
        assert_eq!(on_version.as_deref(), Some(version));
        assert_eq!(on_asset, None);
        assert_eq!(producer, "image.measure-file-size");
        assert_eq!(producer_version, env!("CARGO_PKG_VERSION"));
        let (operator, status, actor): (String, String, String) = sqlx::query_as(
            "SELECT r.operator_id, r.status, u.username FROM public.operation_runs r \
             JOIN public.users u ON u.id = r.actor_user_id \
             WHERE r.asset_version_id = $1::uuid AND r.operator_id = 'image.measure-file-size'",
        )
        .bind(version)
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert_eq!(
            (operator.as_str(), status.as_str(), actor.as_str()),
            ("image.measure-file-size", "succeeded", "sam")
        );
    }

    #[tokio::test]
    async fn upload_of_a_png_records_its_dimensions() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded_ids("nebula.png", PNG).await;
        assert_eq!(instance.facets_of(&id).await, upload_facets(PNG.len()));
    }

    #[tokio::test]
    async fn upload_records_the_header_values_with_their_producer_and_run() {
        let instance = Instance::new().await;
        let (_, version) = instance.uploaded_ids("nebula.png", PNG).await;
        let mut conn = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        let runs: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT r.id::text, r.status, u.username FROM public.operation_runs r \
             JOIN public.users u ON u.id = r.actor_user_id \
             WHERE r.asset_version_id = $1::uuid AND r.operator_id = 'image.read-header'",
        )
        .bind(&version)
        .fetch_all(&mut conn)
        .await
        .unwrap();
        assert_eq!(runs.len(), 1, "{runs:?}");
        let (run, status, actor) = &runs[0];
        assert_eq!((status.as_str(), actor.as_str()), ("succeeded", "sam"));
        let values: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT facet, producer_id, producer_version, run_id::text FROM public.facet_values \
             WHERE asset_version_id = $1::uuid AND facet <> 'image.file-size' ORDER BY facet",
        )
        .bind(&version)
        .fetch_all(&mut conn)
        .await
        .unwrap();
        let expected: Vec<_> = ["image.height", "image.width"]
            .into_iter()
            .map(|facet| {
                (
                    facet.to_owned(),
                    "image.read-header".to_owned(),
                    env!("CARGO_PKG_VERSION").to_owned(),
                    run.clone(),
                )
            })
            .collect();
        assert_eq!(values, expected);
    }

    #[tokio::test]
    async fn upload_of_a_jpeg_records_its_dimensions() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded_ids("nebula.jpg", JPEG).await;
        assert_eq!(instance.facets_of(&id).await, upload_facets(JPEG.len()));
    }

    #[tokio::test]
    async fn upload_of_png_bytes_named_jpg_records_their_png_dimensions() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded_ids("m31.jpg", PNG).await;
        assert_eq!(instance.facets_of(&id).await, upload_facets(PNG.len()));
    }

    /// An Asset saved before the header Operator ran on upload has only its
    /// file size, and nothing runs the header Operator over it later.
    #[tokio::test]
    async fn upload_from_before_the_header_operator_keeps_only_its_file_size() {
        let instance = Instance::with_library(|library| {
            library.with_upload_operators(vec![FILE_SIZE.to_owned()])
        })
        .await;
        let (id, version) = instance.uploaded_ids("nebula.png", PNG).await;
        assert_eq!(
            instance.facets_of(&id).await,
            serde_json::json!([upload_facets(PNG.len())[0]])
        );
        let header_runs: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.operation_runs \
             WHERE asset_version_id = $1::uuid AND operator_id = 'image.read-header'",
        )
        .bind(&version)
        .fetch_one(
            &mut PgConnection::connect_with(&instance.db.options)
                .await
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(header_runs, 0);
    }

    #[tokio::test]
    async fn upload_publishes_the_original_bytes() {
        let instance = Instance::new().await;
        instance.upload("nebula.png", PNG).await;
        let stored = instance.store_files();
        // The original, and its preview.
        assert_eq!(stored.len(), 2);
        instance.object_holding(PNG);
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
        assert_eq!(instance.original_media_type().await, "image/png");
    }

    #[tokio::test]
    async fn upload_of_a_jpeg_records_its_format() {
        let instance = Instance::new().await;
        let response = instance.upload("nebula.jpg", JPEG).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(instance.original_media_type().await, "image/jpeg");
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
        instance.object_holding(PNG);
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
        instance.object_holding(PNG);
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
        assert_eq!(instance.derived_rows().await, 3);
        assert_eq!(instance.store_files().len(), 2);
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
        let path = instance.object_holding(PNG);
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
        let path = instance.object_holding(PNG);
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

/// A pack `test` whose Operators upload runs in place of the file-size
/// Operator. It declares the facet `test.size`, a count in bytes.
struct UploadPack(Mutex<Vec<Box<dyn Operator>>>);
impl Pack for UploadPack {
    fn id(&self) -> &str {
        "test"
    }
    fn register(&self, registry: &mut Registry) {
        registry.declare_facet(FacetSchema {
            name: "test.size".into(),
            label: "Size".into(),
            value_type: FacetType::Count,
            unit: "bytes".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        });
        for operator in self.0.lock().unwrap().drain(..) {
            registry.register_operator(operator);
        }
    }
}

/// A preview runner over the helper built with this server.
fn helper() -> Arc<WorkerRunner> {
    Arc::new(WorkerRunner::new(WorkerCommand {
        program: env!("CARGO_BIN_EXE_sidereal-preview-worker").into(),
        args: Vec::new(),
    }))
}

/// Make upload run `operator`, registered beside the image pack.
fn running(operator: impl Operator + 'static) -> impl FnOnce(Library) -> Library {
    move |library| {
        let id = operator.id().to_owned();
        let mut registry = Registry::new();
        registry
            .register_pack(&ImagePack::new(library.previews()))
            .unwrap();
        registry
            .register_pack(&UploadPack(Mutex::new(vec![Box::new(operator)])))
            .unwrap();
        library
            .with_registry(registry)
            .with_upload_operators(vec![id])
    }
}

/// Make upload run `operator` first, registered beside the image pack, then
/// the file-size and header Operators.
fn running_first(operator: impl Operator + 'static) -> impl FnOnce(Library) -> Library {
    move |library| {
        let id = operator.id().to_owned();
        let mut registry = Registry::new();
        registry
            .register_pack(&ImagePack::new(library.previews()))
            .unwrap();
        registry
            .register_pack(&UploadPack(Mutex::new(vec![Box::new(operator)])))
            .unwrap();
        library.with_registry(registry).with_upload_operators(vec![
            id,
            FILE_SIZE.to_owned(),
            READ_HEADER.to_owned(),
            MAKE_PREVIEW.to_owned(),
        ])
    }
}

impl Instance {
    /// Wait until the Asset's version has a `succeeded` run of each Operator
    /// in `operators`, and holds its file size, width and height.
    async fn eventually_has_every_upload_fact(&self, operators: &[&str]) {
        for operator in operators {
            self.eventually(&format!(
                "SELECT count(*) = 1 FROM public.operation_runs \
                 WHERE operator_id = '{operator}' AND status = 'succeeded'"
            ))
            .await;
        }
        for (facet, value) in [
            ("image.file-size", PNG.len().to_string()),
            ("image.width", "32".to_owned()),
            ("image.height", "24".to_owned()),
        ] {
            self.eventually(&format!(
                "SELECT count(*) = 1 FROM public.facet_values \
                 WHERE facet = '{facet}' AND value = '{value}'::jsonb"
            ))
            .await;
        }
    }
}

/// Proposes the version's size as `test.size`, once `release` is set.
struct Blocking {
    started: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}
impl Operator for Blocking {
    fn id(&self) -> &str {
        "test.block"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn writes(&self) -> &[&str] {
        &["test.size"]
    }
    fn accepts(&self) -> &[&str] {
        &["image/png", "image/jpeg"]
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        self.started.store(true, Ordering::SeqCst);
        let deadline = std::time::Instant::now() + SECONDS;
        while !self.release.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let size = context.candidate_size() as i64;
        context
            .propose_facet("test.size", FacetValue::Integer(size))
            .map_err(|_| OperatorError::Failed("proposal".into()))
    }
}

impl Instance {
    /// The answer to `sql`, a boolean query, once it is true, or a panic
    /// after ten seconds.
    async fn eventually(&self, sql: &str) {
        let mut conn = PgConnection::connect_with(&self.db.options).await.unwrap();
        let deadline = tokio::time::Instant::now() + SECONDS;
        loop {
            let done: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
                .fetch_one(&mut conn)
                .await
                .unwrap();
            if done {
                return;
            }
            assert!(tokio::time::Instant::now() < deadline, "never true: {sql}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// Start uploading `bytes` as `nebula.png` in its own task.
    fn start_upload(&self, bytes: &[u8]) -> tokio::task::JoinHandle<Response> {
        let request = self
            .upload_request()
            .body(Body::from(multipart(
                &[Part::file("file", "nebula.png", "image/png", bytes)],
                true,
                b"",
            )))
            .unwrap();
        let app = self.app.clone();
        tokio::spawn(async move { app.oneshot(request).await.unwrap() })
    }
}

mod upload_disconnect {
    use super::*;

    #[tokio::test]
    async fn upload_disconnect_while_the_asset_commits_still_saves_and_runs() {
        let instance = Instance::new().await;
        let mut locker = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        sqlx::raw_sql("BEGIN; LOCK TABLE public.assets IN ACCESS EXCLUSIVE MODE")
            .execute(&mut locker)
            .await
            .unwrap();
        let upload = instance.start_upload(PNG);
        // The upload's insert waits on the lock: ingest has passed its last
        // cancellation check.
        instance
            .eventually(
                "SELECT count(*) > 0 FROM pg_locks \
                 WHERE relation = 'public.assets'::regclass AND NOT granted",
            )
            .await;
        upload.abort();
        assert!(upload.await.unwrap_err().is_cancelled());
        sqlx::raw_sql("ROLLBACK")
            .execute(&mut locker)
            .await
            .unwrap();
        instance
            .eventually_has_every_upload_fact(&[
                "image.measure-file-size",
                "image.read-header",
                "image.make-preview",
            ])
            .await;
        assert_eq!(instance.rows().await, 2);
        let listed = json(instance.list().await).await;
        let id = listed["assets"][0]["id"].as_str().unwrap();
        assert_eq!(
            listed["assets"][0]["preview_url"],
            format!("/api/assets/{id}/preview")
        );
    }

    #[tokio::test]
    async fn upload_disconnect_during_the_run_still_ends_it_succeeded() {
        let started = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let instance = Instance::with_library(running_first(Blocking {
            started: started.clone(),
            release: release.clone(),
        }))
        .await;
        let upload = instance.start_upload(PNG);
        let deadline = tokio::time::Instant::now() + SECONDS;
        while !started.load(Ordering::SeqCst) {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the run never started"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        upload.abort();
        assert!(upload.await.unwrap_err().is_cancelled());
        let later_runs: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.operation_runs \
             WHERE operator_id IN ('image.measure-file-size', 'image.read-header')",
        )
        .fetch_one(
            &mut PgConnection::connect_with(&instance.db.options)
                .await
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(later_runs, 0);
        release.store(true, Ordering::SeqCst);
        instance
            .eventually_has_every_upload_fact(&[
                "test.block",
                "image.measure-file-size",
                "image.read-header",
                "image.make-preview",
            ])
            .await;
        let listed = json(instance.list().await).await;
        let id = listed["assets"][0]["id"].as_str().unwrap();
        assert_eq!(
            listed["assets"][0]["preview_url"],
            format!("/api/assets/{id}/preview")
        );
        instance
            .eventually(&format!(
                "SELECT count(*) = 1 FROM public.facet_values \
                 WHERE facet = 'test.size' AND value = '{}'::jsonb",
                PNG.len()
            ))
            .await;
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
        let (database, paired, _) =
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
        let (database, paired, _) =
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

mod get_asset {
    use super::*;

    impl Instance {
        async fn read(&self, id: &str) -> Response {
            self.get(&format!("/api/assets/{id}"), Some(&self.cookie))
                .await
        }
    }

    #[tokio::test]
    async fn get_asset_wraps_the_list_entry_with_its_facets() {
        let instance = Instance::new().await;
        instance.upload("nebula.png", PNG).await;
        let listed = json(instance.list().await).await["assets"][0].clone();
        let response = instance.read(listed["id"].as_str().unwrap()).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json(response).await;
        let fields: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
        assert_eq!(fields, ["asset"]);
        let mut asset = body["asset"].clone();
        assert!(asset["facets"].is_array());
        assert_eq!(asset["media_type"], "image/png");
        asset.as_object_mut().unwrap().remove("facets");
        asset.as_object_mut().unwrap().remove("media_type");
        assert_eq!(asset, listed);
    }

    impl Instance {
        /// The Asset `GET /api/assets/{id}` returns for a new upload of
        /// `bytes` as `name`.
        async fn read_upload(&self, name: &str, bytes: &[u8]) -> Value {
            let (id, _) = self.uploaded_ids(name, bytes).await;
            let response = self.read(&id).await;
            assert_eq!(response.status(), StatusCode::OK);
            json(response).await["asset"].clone()
        }
    }

    #[tokio::test]
    async fn get_asset_gives_a_png_its_media_type() {
        let instance = Instance::new().await;
        let asset = instance.read_upload("nebula.png", PNG).await;
        assert_eq!(asset["media_type"], "image/png");
    }

    #[tokio::test]
    async fn get_asset_gives_a_jpeg_its_media_type() {
        let instance = Instance::new().await;
        let asset = instance.read_upload("nebula.jpg", JPEG).await;
        assert_eq!(asset["media_type"], "image/jpeg");
    }

    #[tokio::test]
    async fn get_asset_gives_png_bytes_named_jpg_the_png_media_type() {
        let instance = Instance::new().await;
        let asset = instance.read_upload("m31.jpg", PNG).await;
        assert_eq!(asset["media_type"], "image/png");
        let dimensions: Vec<_> = asset["facets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|facet| facet["name"] != "image.file-size")
            .map(|facet| (facet["name"].clone(), facet["value"].clone()))
            .collect();
        assert_eq!(
            dimensions,
            [
                (serde_json::json!("image.height"), serde_json::json!(24)),
                (serde_json::json!("image.width"), serde_json::json!(32)),
            ]
        );
    }

    #[tokio::test]
    async fn media_type_is_neither_in_the_upload_body_nor_in_the_list() {
        let instance = Instance::new().await;
        let uploaded = json(instance.upload("nebula.png", PNG).await).await;
        assert!(uploaded["asset"].get("media_type").is_none(), "{uploaded}");
        let listed = json(instance.list().await).await;
        assert_eq!(listed["assets"][0]["id"], uploaded["asset"]["id"]);
        assert!(listed["assets"][0].get("media_type").is_none(), "{listed}");
    }

    #[tokio::test]
    async fn get_asset_returns_the_file_size_without_provenance() {
        let instance = Instance::new().await;
        let uploaded = json(instance.upload("nebula.png", &png_of_size(1_234_567)).await).await;
        let response = instance
            .read(uploaded["asset"]["id"].as_str().unwrap())
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json(response).await["asset"]["facets"],
            upload_facets(1_234_567)
        );
    }

    #[tokio::test]
    async fn get_asset_with_no_facets_has_an_empty_list() {
        let instance =
            Instance::with_library(|library| library.with_upload_operators(vec![])).await;
        let uploaded = json(instance.upload("nebula.png", PNG).await).await;
        let response = instance
            .read(uploaded["asset"]["id"].as_str().unwrap())
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json(response).await["asset"]["facets"],
            serde_json::json!([])
        );
    }

    /// Values of the Asset and its current version are listed by name, then
    /// write time, then id, each with its own schema version's label and
    /// unit; a value of another version is not.
    #[tokio::test]
    async fn get_asset_orders_facets_and_reads_each_with_its_own_schema() {
        let instance =
            Instance::with_library(|library| library.with_upload_operators(vec![])).await;
        let uploaded = json(instance.upload("nebula.png", PNG).await).await;
        let asset = uploaded["asset"]["id"].as_str().unwrap().to_owned();
        let version = uploaded["asset"]["current_version_id"]
            .as_str()
            .unwrap()
            .to_owned();
        instance
            .execute(&format!(
                "INSERT INTO public.asset_versions (id, asset_id, content_hash, byte_size, media_type, source_id, source_version, ingested_at) \
                 SELECT '00000000-0000-4000-8000-0000000000f1', asset_id, content_hash, byte_size, media_type, source_id, source_version, ingested_at \
                 FROM public.asset_versions WHERE id = '{version}';
                 INSERT INTO public.operation_runs (id, operator_id, operator_version, asset_version_id, actor_user_id, status, started_at, ended_at) \
                 SELECT '00000000-0000-4000-8000-0000000000e1', 'test.op', '1', '{version}', id, 'succeeded', now(), now() FROM public.users;
                 INSERT INTO public.operation_runs (id, operator_id, operator_version, asset_version_id, actor_user_id, status, started_at, ended_at) \
                 SELECT '00000000-0000-4000-8000-0000000000e2', 'test.op', '1', '00000000-0000-4000-8000-0000000000f1', id, 'succeeded', now(), now() FROM public.users;
                 INSERT INTO public.facet_schemas VALUES \
                   ('test.b', 1, 'test', 'B one', 'count', 'px', 'asset_version'), \
                   ('test.b', 2, 'test', 'B two', 'count', 'mm', 'asset_version'), \
                   ('test.a', 1, 'test', 'A', 'count', '', 'asset');
                 INSERT INTO public.facet_values (id, facet, schema_version, asset_id, asset_version_id, value, producer_id, producer_version, run_id, written_at) VALUES \
                   ('00000000-0000-4000-8000-000000000003', 'test.b', 2, NULL, '{version}', '3', 'test.op', '1', '00000000-0000-4000-8000-0000000000e1', '2026-01-02T00:00:00Z'), \
                   ('00000000-0000-4000-8000-000000000002', 'test.b', 1, NULL, '{version}', '2', 'test.op', '1', '00000000-0000-4000-8000-0000000000e1', '2026-01-01T00:00:00Z'), \
                   ('00000000-0000-4000-8000-000000000005', 'test.b', 1, NULL, '{version}', '5', 'test.op', '1', '00000000-0000-4000-8000-0000000000e1', '2026-01-02T00:00:00Z'), \
                   ('00000000-0000-4000-8000-000000000001', 'test.a', 1, '{asset}', NULL, '1', 'test.op', '1', '00000000-0000-4000-8000-0000000000e1', '2026-01-03T00:00:00Z'), \
                   ('00000000-0000-4000-8000-000000000009', 'test.b', 1, NULL, '00000000-0000-4000-8000-0000000000f1', '9', 'test.op', '1', '00000000-0000-4000-8000-0000000000e2', '2026-01-01T00:00:00Z');"
            ))
            .await;
        let response = instance.read(&asset).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json(response).await["asset"]["facets"],
            serde_json::json!([
                {"name": "test.a", "label": "A", "value": 1, "unit": ""},
                {"name": "test.b", "label": "B one", "value": 2, "unit": "px"},
                {"name": "test.b", "label": "B two", "value": 3, "unit": "mm"},
                {"name": "test.b", "label": "B one", "value": 5, "unit": "px"},
            ])
        );
    }

    #[tokio::test]
    async fn get_asset_of_an_unknown_or_malformed_id_is_404() {
        let instance = Instance::new().await;
        instance.upload("nebula.png", PNG).await;
        for id in ["00000000-0000-4000-8000-000000000000", "not-an-id"] {
            let response = instance.read(id).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{id}");
            let body = body_text(response).await;
            assert!(
                !body.contains("nebula") && !body.contains("facets"),
                "{id}: {body}"
            );
        }
    }

    #[tokio::test]
    async fn get_asset_answers_503_when_the_database_fails_after_the_session_check() {
        for broken in ["assets", "facet_values"] {
            let instance = Instance::new().await;
            let uploaded = json(instance.upload("nebula.png", PNG).await).await;
            instance
                .execute(&format!("ALTER TABLE public.{broken} RENAME TO moved_away"))
                .await;
            let response = instance
                .read(uploaded["asset"]["id"].as_str().unwrap())
                .await;
            assert_eq!(
                response.status(),
                StatusCode::SERVICE_UNAVAILABLE,
                "{broken}"
            );
            assert_eq!(
                body_text(response).await,
                r#"{"error":"the library is unavailable; try again later"}"#,
                "{broken}"
            );
        }
    }

    #[tokio::test]
    async fn get_asset_without_a_valid_session_answers_401_with_no_metadata() {
        for (case, sql) in INVALID_SESSIONS {
            let instance = Instance::new().await;
            let uploaded = json(instance.upload("nebula.png", PNG).await).await;
            let cookie = match sql {
                Some(sql) => Some(instance.with_session(sql).await),
                None => None,
            };
            let id = uploaded["asset"]["id"].as_str().unwrap();
            let response = instance.get(&format!("/api/assets/{id}"), cookie).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{case}");
            let body = body_text(response).await;
            assert!(
                !body.contains("nebula") && !body.contains("facets"),
                "{case}: {body}"
            );
        }
    }

    #[tokio::test]
    async fn get_asset_from_another_origin_is_not_refused() {
        let instance = Instance::new().await;
        let uploaded = json(instance.upload("nebula.png", PNG).await).await;
        let id = uploaded["asset"]["id"].as_str().unwrap();
        let request = Request::get(format!("/api/assets/{id}"))
            .header(header::COOKIE, &instance.cookie)
            .header(header::HOST, "photos.example")
            .header(header::ORIGIN, "https://elsewhere.example")
            .body(Body::empty())
            .unwrap();
        assert_eq!(instance.send(request).await.status(), StatusCode::OK);
    }
}

mod upload_run {
    use super::*;

    /// An Operator `test.op` that writes `test.size`, run by `script`.
    struct Scripted(fn(&mut AssetContext<'_>) -> Result<(), OperatorError>);
    impl Operator for Scripted {
        fn id(&self) -> &str {
            "test.op"
        }
        fn version(&self) -> &str {
            "1"
        }
        fn writes(&self) -> &[&str] {
            &["test.size"]
        }
        fn accepts(&self) -> &[&str] {
            &["image/png", "image/jpeg"]
        }
        fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            (self.0)(context)
        }
    }

    /// Upload `nebula.png` to a server whose upload runs `script`; check the
    /// 201 and its body against the library list; answer the Asset's id.
    async fn uploaded(instance: &Instance) -> String {
        let response = instance.upload("nebula.png", PNG).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = json(response).await;
        let listed = json(instance.list().await).await["assets"].clone();
        assert_eq!(listed, serde_json::json!([body["asset"]]));
        body["asset"]["id"].as_str().unwrap().to_owned()
    }

    async fn facets(instance: &Instance, id: &str) -> Value {
        let response = instance
            .get(&format!("/api/assets/{id}"), Some(&instance.cookie))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        json(response).await["asset"]["facets"].clone()
    }

    /// The one run's status and reason.
    async fn run(instance: &Instance) -> (String, Option<String>) {
        let mut conn = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        sqlx::query_as("SELECT status, reason FROM public.operation_runs")
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn upload_run_that_fails_still_answers_201() {
        let instance = Instance::with_library(running(Scripted(|_| {
            Err(OperatorError::Failed("could not read the header".into()))
        })))
        .await;
        let id = uploaded(&instance).await;
        assert_eq!(facets(&instance, &id).await, serde_json::json!([]));
        assert_eq!(
            run(&instance).await,
            ("failed".into(), Some("could not read the header".into()))
        );
        let line = format!("test.op run failed for asset {id}: could not read the header");
        assert!(instance.logged().contains(&line), "{:?}", instance.logged());
    }

    #[tokio::test]
    async fn upload_run_whose_operator_panics_still_answers_201() {
        let instance =
            Instance::with_library(running(Scripted(|_| panic!("the operator broke")))).await;
        let id = uploaded(&instance).await;
        assert_eq!(
            run(&instance).await,
            ("failed".into(), Some("the operator stopped".into()))
        );
        // The server answers the next request.
        assert_eq!(facets(&instance, &id).await, serde_json::json!([]));
        assert_eq!(instance.list().await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn upload_run_whose_proposal_is_refused_still_answers_201() {
        let instance = Instance::with_library(running(Scripted(|context| {
            context
                .propose_facet("test.size", FacetValue::Integer(-1))
                .map_err(|_| OperatorError::Failed("proposal".into()))
        })))
        .await;
        let id = uploaded(&instance).await;
        assert_eq!(facets(&instance, &id).await, serde_json::json!([]));
        let (status, reason) = run(&instance).await;
        assert_eq!(status, "failed");
        assert!(reason.unwrap().contains("test.size"));
    }

    #[tokio::test]
    async fn upload_run_whose_first_insert_is_refused_still_answers_201() {
        let instance = Instance::new().await;
        instance
            .execute("ALTER TABLE public.operation_runs ADD CONSTRAINT refuse_all CHECK (false)")
            .await;
        let id = uploaded(&instance).await;
        assert_eq!(instance.rows().await, 2);
        assert_eq!(facets(&instance, &id).await, serde_json::json!([]));
        assert_not_recorded(
            &instance,
            &id,
            &["image.measure-file-size", "image.read-header"],
        );
    }

    /// Every update of a run is refused, so core cannot record or read back
    /// how it ended: the outcome is unknown, and only the answer and the log
    /// line are certain.
    #[tokio::test]
    async fn upload_run_whose_outcome_is_unknown_still_answers_201() {
        let instance = Instance::new().await;
        instance
            .execute(
                "CREATE FUNCTION public.refuse_update() RETURNS trigger LANGUAGE plpgsql \
                 AS $$ BEGIN RAISE EXCEPTION 'refused'; END $$; \
                 CREATE TRIGGER refuse_update BEFORE UPDATE ON public.operation_runs \
                 FOR EACH ROW EXECUTE FUNCTION public.refuse_update();",
            )
            .await;
        let response = instance.upload("nebula.png", PNG).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let id = json(response).await["asset"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(instance.rows().await, 2);
        assert_not_recorded(
            &instance,
            &id,
            &["image.measure-file-size", "image.read-header"],
        );
    }

    /// The log has a `<operator> run not recorded for asset <id>:` line for
    /// each of `operators`.
    fn assert_not_recorded(instance: &Instance, id: &str, operators: &[&str]) {
        let logged = instance.logged();
        for operator in operators {
            let prefix = format!("{operator} run not recorded for asset {id}:");
            assert!(
                logged.iter().any(|line| line.starts_with(&prefix)),
                "{prefix} {logged:?}"
            );
        }
    }

    /// 1,000 bytes that start with the JPEG signature: the Source accepts
    /// them, and the header Operator cannot read a frame header.
    fn start_of_a_jpeg() -> Vec<u8> {
        let mut bytes =
            b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00".to_vec();
        bytes.resize(1_000, 0);
        bytes
    }

    /// Every run of the Asset's version, by Operator id: its status and
    /// reason.
    async fn runs(instance: &Instance) -> Vec<(String, String, Option<String>)> {
        let mut conn = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        sqlx::query_as(
            "SELECT operator_id, status, reason FROM public.operation_runs ORDER BY operator_id",
        )
        .fetch_all(&mut conn)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn upload_run_of_an_unreadable_header_still_records_the_file_size() {
        let instance = Instance::new().await;
        let (id, _) = instance.uploaded_ids("m31.jpg", &start_of_a_jpeg()).await;
        assert_eq!(
            instance.facets_of(&id).await,
            serde_json::json!([upload_facets(1_000)[0]])
        );
        assert_eq!(
            runs(&instance).await,
            [
                (
                    "image.make-preview".into(),
                    "failed".into(),
                    Some("the JPEG image could not be decoded".into())
                ),
                ("image.measure-file-size".into(), "succeeded".into(), None),
                (
                    "image.read-header".into(),
                    "failed".into(),
                    Some("the JPEG header could not be read".into())
                ),
            ]
        );
        let line = format!(
            "image.read-header run failed for asset {id}: the JPEG header could not be read"
        );
        assert!(instance.logged().contains(&line), "{:?}", instance.logged());
    }

    #[tokio::test]
    async fn upload_run_of_an_unreadable_header_whose_outcome_cannot_be_recorded_answers_201() {
        let instance = Instance::new().await;
        instance
            .execute(
                "CREATE FUNCTION public.refuse_update() RETURNS trigger LANGUAGE plpgsql \
                 AS $$ BEGIN RAISE EXCEPTION 'refused'; END $$; \
                 CREATE TRIGGER refuse_update BEFORE UPDATE ON public.operation_runs \
                 FOR EACH ROW EXECUTE FUNCTION public.refuse_update();",
            )
            .await;
        let (id, _) = instance.uploaded_ids("m31.jpg", &start_of_a_jpeg()).await;
        assert_eq!(instance.rows().await, 2);
        let header_values: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.facet_values \
             WHERE facet IN ('image.width', 'image.height')",
        )
        .fetch_one(
            &mut PgConnection::connect_with(&instance.db.options)
                .await
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(header_values, 0);
        let header_run = runs(&instance)
            .await
            .into_iter()
            .find(|run| run.0 == "image.read-header")
            .unwrap();
        assert_eq!((header_run.1.as_str(), header_run.2), ("running", None));
        assert_not_recorded(&instance, &id, &["image.read-header"]);
    }

    #[tokio::test]
    async fn upload_run_of_the_file_size_that_fails_still_records_the_header() {
        let instance = Instance::new().await;
        instance
            .execute(
                "CREATE FUNCTION public.refuse_file_size() RETURNS trigger LANGUAGE plpgsql \
                 AS $$ BEGIN \
                   IF NEW.facet = 'image.file-size' THEN RAISE EXCEPTION 'refused'; END IF; \
                   RETURN NEW; \
                 END $$; \
                 CREATE TRIGGER refuse_file_size BEFORE INSERT ON public.facet_values \
                 FOR EACH ROW EXECUTE FUNCTION public.refuse_file_size();",
            )
            .await;
        let (id, _) = instance.uploaded_ids("nebula.png", PNG).await;
        assert_eq!(
            instance.facets_of(&id).await,
            serde_json::json!(upload_facets(PNG.len()).as_array().unwrap()[1..])
        );
        assert_eq!(
            runs(&instance).await,
            [
                ("image.make-preview".into(), "succeeded".into(), None),
                (
                    "image.measure-file-size".into(),
                    "failed".into(),
                    Some("the values could not be stored".into())
                ),
                ("image.read-header".into(), "succeeded".into(), None),
            ]
        );
        let line = format!(
            "image.measure-file-size run failed for asset {id}: the values could not be stored"
        );
        assert!(instance.logged().contains(&line), "{:?}", instance.logged());
    }
}

mod preview_upload {
    use super::*;
    use sidereal_plugin_abi::{Source, SourceError};
    use tiff::{Image, Tiff};

    /// An 8-bit RGB TIFF of `width` by `height`.
    fn rgb_tiff(width: u64, height: u64) -> Vec<u8> {
        Tiff::classic(vec![Image::rgb8(width, height, |x, y| {
            [(x % 256) as u8, (y % 256) as u8, 90]
        })])
        .bytes()
    }

    /// An 8-bit CMYK TIFF of 800 by 600, which gets no preview.
    fn cmyk_tiff() -> Vec<u8> {
        Tiff::classic(vec![Image::chunky(
            800,
            600,
            8,
            4,
            5,
            vec![40; 800 * 600 * 4],
        )])
        .bytes()
    }

    /// Every run, in the order the runs started: its Operator, status,
    /// reason, actor and whether it ended no earlier than the run before.
    async fn runs_in_order(instance: &Instance) -> Vec<(String, String, Option<String>, i64)> {
        let mut conn = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        let in_order: bool = sqlx::query_scalar(
            "SELECT coalesce(bool_and(ended_at >= previous), true) FROM ( \
             SELECT ended_at, lag(ended_at) OVER (ORDER BY started_at, ended_at) AS previous \
             FROM public.operation_runs) r WHERE previous IS NOT NULL",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert!(in_order, "a run ended before the run started before it");
        sqlx::query_as(
            "SELECT operator_id, status, reason, actor_user_id FROM public.operation_runs \
             ORDER BY started_at, ended_at",
        )
        .fetch_all(&mut conn)
        .await
        .unwrap()
    }

    async fn uploader(instance: &Instance) -> i64 {
        instance
            .count("SELECT id FROM public.users WHERE username = 'sam'")
            .await
    }

    #[tokio::test]
    async fn upload_of_an_8_bit_rgb_tiff_answers_201_with_its_preview() {
        let instance = Instance::new().await;
        let response = instance.upload("m31.tif", &rgb_tiff(800, 600)).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let asset = json(response).await["asset"].clone();
        let id = asset["id"].as_str().unwrap();
        assert_eq!(asset["preview_url"], format!("/api/assets/{id}/preview"));
        let sam = uploader(&instance).await;
        let previews: Vec<_> = runs_in_order(&instance)
            .await
            .into_iter()
            .filter(|run| run.0 == MAKE_PREVIEW)
            .collect();
        assert_eq!(
            previews,
            [(MAKE_PREVIEW.to_owned(), "succeeded".to_owned(), None, sam)]
        );
        let read = json(
            instance
                .get(&format!("/api/assets/{id}"), Some(&instance.cookie))
                .await,
        )
        .await;
        assert_eq!(read["asset"]["media_type"], "image/tiff");
        let facts: Vec<(String, i64)> = read["asset"]["facets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["name"].as_str().unwrap().to_owned(),
                    f["value"].as_i64().unwrap(),
                )
            })
            .filter(|(name, _)| name != "image.file-size")
            .collect();
        assert_eq!(
            facts,
            [
                ("image.height".to_owned(), 600),
                ("image.width".to_owned(), 800)
            ]
        );
    }

    #[tokio::test]
    async fn upload_of_a_png_a_jpeg_and_a_tiff_runs_each_upload_operator_once_in_order() {
        for (name, bytes) in [
            ("nebula.png", PNG.to_vec()),
            ("nebula.jpg", JPEG.to_vec()),
            ("m31.tif", rgb_tiff(64, 48)),
        ] {
            let instance = Instance::new().await;
            assert_eq!(
                instance.upload(name, &bytes).await.status(),
                StatusCode::CREATED
            );
            let operators: Vec<(String, String)> = runs_in_order(&instance)
                .await
                .into_iter()
                .map(|run| (run.0, run.1))
                .collect();
            assert_eq!(
                operators,
                [FILE_SIZE, READ_HEADER, MAKE_PREVIEW]
                    .map(|o| (o.to_owned(), "succeeded".to_owned())),
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn upload_of_nebula_png_gets_a_jpeg_preview_with_no_runs_of_its_own() {
        let instance = Instance::new().await;
        instance.upload("nebula.png", PNG).await;
        let mut conn = PgConnection::connect_with(&instance.db.options)
            .await
            .unwrap();
        let (media_type, hash): (String, String) = sqlx::query_as(
            "SELECT v.media_type, v.content_hash FROM public.asset_lineage l \
             JOIN public.asset_versions v ON v.id = l.output_version_id",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert_eq!(media_type, "image/jpeg");
        let path = instance
            .storage()
            .join("store")
            .join(&hash[..2])
            .join(&hash[2..4])
            .join(&hash);
        assert_eq!(&std::fs::read(path).unwrap()[..2], [0xff, 0xd8]);
        let preview_runs = instance
            .count(
                "SELECT count(*) FROM public.operation_runs r \
                 JOIN public.asset_lineage l ON l.output_version_id = r.asset_version_id",
            )
            .await;
        assert_eq!(preview_runs, 0);
        let preview_facts = instance
            .count(
                "SELECT count(*) FROM public.facet_values f \
                 JOIN public.asset_lineage l ON l.output_version_id = f.asset_version_id",
            )
            .await;
        assert_eq!(preview_facts, 0);
        let previews_of_previews = instance
            .count(
                "SELECT count(*) FROM public.asset_lineage a \
                 JOIN public.asset_lineage b ON b.input_version_id = a.output_version_id",
            )
            .await;
        assert_eq!(previews_of_previews, 0);
    }

    #[tokio::test]
    async fn upload_of_a_cmyk_tiff_answers_201_with_no_preview_and_a_failed_run() {
        let instance = Instance::new().await;
        let response = instance.upload("m31.tif", &cmyk_tiff()).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let first = json(response).await["asset"].clone();
        assert_eq!(first["preview_url"], Value::Null);
        let preview = runs_in_order(&instance)
            .await
            .into_iter()
            .find(|run| run.0 == MAKE_PREVIEW)
            .unwrap();
        assert_eq!(
            (preview.1.as_str(), preview.2.as_deref()),
            ("failed", Some("the TIFF image type is not supported"))
        );
        let again = instance.upload("again.tif", &cmyk_tiff()).await;
        assert_eq!(again.status(), StatusCode::CONFLICT);
        let found = json(again).await["asset"].clone();
        assert_eq!(found, first);
        assert_eq!(found["preview_url"], Value::Null);
    }

    #[tokio::test]
    async fn upload_of_the_same_bytes_again_runs_nothing_and_answers_the_first_preview_url() {
        let instance = Instance::new().await;
        let first = json(instance.upload("nebula.png", PNG).await).await["asset"].clone();
        let runs = instance
            .count("SELECT count(*) FROM public.operation_runs")
            .await;
        let response = instance.upload("copy.png", PNG).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let found = json(response).await["asset"].clone();
        assert_eq!(found["preview_url"], first["preview_url"]);
        assert!(found["preview_url"].is_string());
        assert_eq!(
            instance
                .count("SELECT count(*) FROM public.operation_runs")
                .await,
            runs
        );
        assert_eq!(instance.derived_rows().await, 3);
    }

    const REFUSAL: &str = "only PNG, JPEG and TIFF images are supported";

    #[tokio::test]
    async fn refused_gif_svg_empty_and_text_answer_415_with_the_packs_reason() {
        let instance = Instance::new().await;
        for (name, bytes) in [
            ("nebula.gif", &b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"[..]),
            ("nebula.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
            ("empty.tif", b""),
            ("nebula.tif", b"plain text, not a TIFF"),
        ] {
            let response = instance.upload(name, bytes).await;
            assert_eq!(
                response.status(),
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "{name}"
            );
            assert_eq!(
                json(response).await,
                serde_json::json!({
                    "error": format!("unsupported format: {REFUSAL}"),
                    "reason": REFUSAL,
                }),
                "{name}"
            );
        }
        assert_eq!(instance.rows().await, 0);
        assert!(instance.store_files().is_empty());
    }

    /// A manual-upload Source that takes only FITS files, and so refuses
    /// every test file.
    struct FitsOnly;
    impl Source for FitsOnly {
        fn id(&self) -> &str {
            sidereal_pack_image::MANUAL_UPLOAD
        }
        fn version(&self) -> &str {
            "1"
        }
        fn run(&self, _: &mut AssetContext<'_>) -> Result<(), SourceError> {
            Err(SourceError::Refused("only FITS files are supported".into()))
        }
    }

    #[tokio::test]
    async fn refused_by_another_source_answers_415_with_that_sources_reason() {
        let instance = Instance::with_library(|library| {
            let mut registry = Registry::new();
            registry.register_source(Box::new(FitsOnly));
            library
                .with_registry(registry)
                .with_upload_operators(Vec::new())
        })
        .await;
        let response = instance.upload("nebula.png", PNG).await;
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            json(response).await,
            serde_json::json!({
                "error": "unsupported format: only FITS files are supported",
                "reason": "only FITS files are supported",
            })
        );
    }

    /// The lines of `path`, or none before it exists.
    fn lines(path: &Path) -> usize {
        std::fs::read_to_string(path).map_or(0, |text| text.lines().count())
    }

    #[tokio::test]
    async fn upload_previews_overlap_one_at_a_time() {
        let scratch = Dir::new();
        let launches = scratch.0.join("launches");
        let release = scratch.0.join("release");
        // A helper that logs its launch, waits for `release`, then answers
        // its input as the preview's bytes.
        let script = format!(
            "echo launched >> '{}'; while [ ! -f '{}' ]; do sleep 0.05; done; exec cat",
            launches.display(),
            release.display()
        );
        let runner = Arc::new(WorkerRunner::new(WorkerCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into(), "worker".into()],
        }));
        let shared = runner.clone();
        let instance = Instance::with_library(move |library| library.with_previews(shared)).await;
        let first = instance.start_upload(PNG);
        let deadline = tokio::time::Instant::now() + SECONDS;
        while lines(&launches) < 1 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the first helper never started"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let second = instance.start_upload(JPEG);
        // The second upload's other runs end; its preview waits for the slot.
        instance
            .eventually(
                "SELECT count(*) = 2 FROM public.operation_runs \
                 WHERE operator_id = 'image.read-header' AND status <> 'running'",
            )
            .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(
            runner.copies_begun(),
            1,
            "the second preview copied its input"
        );
        assert_eq!(lines(&launches), 1, "the second helper launched");
        assert!(!second.is_finished());
        std::fs::write(&release, b"").unwrap();
        let first = first.await.unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
        assert!(json(first).await["asset"]["preview_url"].is_string());
        let second = second.await.unwrap();
        assert_eq!(second.status(), StatusCode::CREATED);
        assert!(json(second).await["asset"]["preview_url"].is_string());
        assert_eq!(runner.copies_begun(), 2);
        assert_eq!(lines(&launches), 2);
    }
}
