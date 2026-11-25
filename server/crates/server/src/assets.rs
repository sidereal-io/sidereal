//! The Asset routes. They sit behind the same origin check and session guard
//! as every other `/api` route (`access::routes`).
//!
//! Upload streams one multipart field into core's staging in bounded chunks,
//! then core runs the manual-upload Source, publishes the object and commits
//! the Asset. Every refusal has a fixed body, so no path or driver error
//! reaches a client; the operational log names the cause.
use crate::access::Log;
use axum::{
    body::{Body, BodyDataStream, Bytes},
    extract::{
        multipart::MultipartError, DefaultBodyLimit, FromRequest, Multipart, Path, Request, State,
    },
    http::{
        header::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
        StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{Stream, StreamExt};
use http_body_util::LengthLimitError;
use serde_json::{json, Value};
use sidereal_core::{
    assets::AssetRecord,
    database::Database,
    ingest::{ingest, IngestError},
    storage::objects::{ObjectStore, StageError, Staging, MAX_UPLOAD_BYTES},
};
use sidereal_pack_astro::AstroPack;
use sidereal_pack_image::{ImagePack, MANUAL_UPLOAD};
use sidereal_plugin_abi::{Pack, Registry};
use std::{
    convert::Infallible,
    io::Read,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};
use tower_http::limit::RequestBodyLimitLayer;

/// The most bytes one upload request may hold, multipart framing and any
/// epilogue included: the 100 MiB file plus 1 MiB.
pub const TRANSPORT_LIMIT: usize = 105_906_176;

/// Staged writes go to disk in chunks of about this size.
const WRITE_CHUNK: usize = 1024 * 1024;

/// Turns an opened object into the reader an original is streamed from.
pub type OriginalReader = Arc<dyn Fn(std::fs::File) -> Box<dyn Read + Send> + Send + Sync>;

/// What the Asset routes work with: the object store, when the server has
/// managed storage, and the Registry of every compiled-in pack.
#[derive(Clone)]
pub struct Library {
    objects: Option<ObjectStore>,
    registry: Arc<Registry>,
    reader: OriginalReader,
}

impl Library {
    /// The production Registry, with `objects` when `STORAGE_ROOT` is set.
    pub fn new(objects: Option<ObjectStore>) -> Self {
        let mut registry = Registry::new();
        AstroPack.register(&mut registry);
        ImagePack.register(&mut registry);
        Self {
            objects,
            registry: Arc::new(registry),
            reader: Arc::new(|file| Box::new(file)),
        }
    }

    /// Read originals through `reader`, so a test can make a read fail
    /// after the response has started.
    #[doc(hidden)]
    pub fn with_original_reader(self, reader: OriginalReader) -> Self {
        Self { reader, ..self }
    }
}

#[derive(Clone)]
struct Assets {
    database: Database,
    library: Library,
    log: Log,
}

/// The Asset routes, to be merged before the access layers.
pub(crate) fn routes(database: Database, library: Library, log: Log) -> Router<Database> {
    Router::new()
        .route(
            "/api/assets",
            post(upload)
                // Content-Length is checked before the body is read; the
                // extractor limit covers a body without one.
                .layer::<_, Infallible>(DefaultBodyLimit::max(TRANSPORT_LIMIT))
                .layer::<_, Infallible>(RequestBodyLimitLayer::new(TRANSPORT_LIMIT))
                .get(list),
        )
        .route("/api/assets/{id}/original", get(original))
        .with_state(Assets {
            database,
            library,
            log,
        })
}

/// One Asset as the API shows it.
fn entry(record: &AssetRecord) -> Value {
    json!({
        "id": record.id.to_string(),
        "kind": record.kind,
        "name": record.name,
        "current_version_id": record.current_version_id.to_string(),
        "original_url": format!("/api/assets/{}/original", record.id),
    })
}

/// Every committed Asset. A database failure is 503, never an empty list.
async fn list(State(assets): State<Assets>) -> Response {
    match assets.database.assets().await {
        Ok(records) => {
            let entries: Vec<Value> = records.iter().map(entry).collect();
            Json(json!({ "assets": entries })).into_response()
        }
        Err(error) => {
            assets.log.line(&format!("library listing failed: {error}"));
            unavailable()
        }
    }
}

/// The bytes stored at the current version's hash path. Every failure found
/// before the response starts is 503; a read that fails later ends the body
/// early, so the client never receives a complete 200.
async fn original(State(assets): State<Assets>, Path(id): Path<String>) -> Response {
    let not_found =
        || (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response();
    let Ok(id) = uuid::Uuid::try_parse(&id) else {
        return not_found();
    };
    let record = match assets.database.asset(id).await {
        Ok(Some(record)) => record,
        Ok(None) => return not_found(),
        Err(error) => {
            assets.log.line(&format!("original lookup failed: {error}"));
            return unavailable();
        }
    };
    // An Asset is found first, so an unknown id is 404 even without storage.
    let Some(objects) = assets.library.objects.clone() else {
        return unavailable();
    };
    let hash = record.version.content_hash.clone();
    let opened = tokio::task::spawn_blocking(move || objects.open_object(&hash)).await;
    let (file, size) = match opened {
        Ok(Ok(opened)) => opened,
        Ok(Err(error)) => {
            assets
                .log
                .line(&format!("original unavailable for asset {id}: {error}"));
            return unavailable();
        }
        Err(_) => return unavailable(),
    };
    let reader = (assets.library.reader)(file);
    let log = assets.log.clone();
    let chunks = futures_util::stream::unfold(Some(reader), move |reader| {
        let log = log.clone();
        async move {
            let mut reader = reader?;
            let read = tokio::task::spawn_blocking(move || {
                let mut buf = vec![0; 64 * 1024];
                let read = reader.read(&mut buf);
                (reader, buf, read)
            })
            .await;
            match read {
                Ok((_, _, Ok(0))) => None,
                Ok((reader, mut buf, Ok(n))) => {
                    buf.truncate(n);
                    Some((Ok(Bytes::from(buf)), Some(reader)))
                }
                Ok((_, _, Err(error))) => {
                    log.line(&format!("original read failed for asset {id}: {error}"));
                    Some((Err(error), None))
                }
                Err(_) => Some((Err(std::io::Error::other("the read task stopped")), None)),
            }
        }
    });
    (
        [
            (CONTENT_TYPE, record.version.media_type.clone()),
            (CONTENT_LENGTH, size.to_string()),
            (X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
            (CACHE_CONTROL, "no-store".to_owned()),
        ],
        Body::from_stream(chunks),
    )
        .into_response()
}

/// The fixed answer for a library read the database could not serve.
fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "the library is unavailable; try again later" })),
    )
        .into_response()
}

/// Every refusal an upload can get, each with a fixed message.
#[derive(Debug)]
enum UploadError {
    Malformed,
    TooLarge,
    Unsupported,
    NoStorage,
    Failed,
}

impl IntoResponse for UploadError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::Malformed => (
                StatusCode::BAD_REQUEST,
                "invalid upload: send one file, with a file name, in a multipart field named file",
            ),
            Self::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "the file is larger than the 100 MiB limit",
            ),
            Self::Unsupported => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported format: only PNG and JPEG images are supported",
            ),
            Self::NoStorage => (
                StatusCode::SERVICE_UNAVAILABLE,
                "storage unavailable: this server has no managed storage",
            ),
            Self::Failed => (
                StatusCode::SERVICE_UNAVAILABLE,
                "the upload could not be saved; try again later",
            ),
        };
        (status, Json(json!({ "error": error }))).into_response()
    }
}

fn multipart_error(error: MultipartError) -> UploadError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE || past_limit(&error) {
        UploadError::TooLarge
    } else {
        UploadError::Malformed
    }
}

/// Whether `error`, or any error it wraps, is the transport limit's.
fn past_limit(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut next = Some(error);
    while let Some(error) = next {
        if error.is::<LengthLimitError>() {
            return true;
        }
        next = error.source();
    }
    false
}

/// The request body, shared by the multipart reader and the epilogue drain
/// that follows it. The lock is held only while one chunk is polled.
#[derive(Clone)]
struct SharedBody(Arc<Mutex<BodyDataStream>>);

impl Stream for SharedBody {
    type Item = Result<Bytes, axum::BoxError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut body = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // The inner error is passed on unwrapped, so the multipart reader
        // still finds the transport limit's error one level down.
        body.poll_next_unpin(cx)
            .map(|chunk| chunk.map(|chunk| chunk.map_err(axum::Error::into_inner)))
    }
}

/// Read the epilogue after the closing delimiter, ignoring its bytes, so
/// that a body past the transport limit gets 413 even without
/// Content-Length. Any other error there leaves the complete file as it is.
async fn drain(mut body: SharedBody) -> Result<(), UploadError> {
    while let Some(chunk) = body.next().await {
        if let Err(error) = chunk {
            return if past_limit(error.as_ref()) {
                Err(UploadError::TooLarge)
            } else {
                Ok(())
            };
        }
    }
    Ok(())
}

/// Marks the ingest cancelled when the request's future is dropped, as when
/// the client goes away, so nothing is published after that.
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

async fn upload(
    State(assets): State<Assets>,
    request: Request,
) -> Result<(StatusCode, Json<Value>), UploadError> {
    let objects = assets
        .library
        .objects
        .clone()
        .ok_or(UploadError::NoStorage)?;
    let (parts, body) = request.into_parts();
    let body = SharedBody(Arc::new(Mutex::new(body.into_data_stream())));
    let request = Request::from_parts(parts, Body::from_stream(body.clone()));
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|_| UploadError::Malformed)?;
    let mut field = multipart
        .next_field()
        .await
        .map_err(multipart_error)?
        .ok_or(UploadError::Malformed)?;
    let name = match (field.name(), field.file_name()) {
        (Some("file"), Some(name)) if !name.is_empty() => name.to_owned(),
        _ => return Err(UploadError::Malformed),
    };
    let failed = |error: &dyn std::fmt::Display| {
        assets.log.line(&format!("upload failed: {error}"));
        UploadError::Failed
    };
    let staging = blocking(move || objects.begin())
        .await?
        .map_err(|e| failed(&e))?;
    let mut staging = Some(staging);
    let mut pending = Vec::with_capacity(WRITE_CHUNK);
    let mut total: u64 = 0;
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        total += chunk.len() as u64;
        if total > MAX_UPLOAD_BYTES {
            return Err(UploadError::TooLarge);
        }
        pending.extend_from_slice(&chunk);
        if pending.len() >= WRITE_CHUNK {
            staging = Some(
                write(
                    staging.take().unwrap(),
                    std::mem::take(&mut pending),
                    &failed,
                )
                .await?,
            );
        }
    }
    let staging = write(staging.take().unwrap(), pending, &failed).await?;
    // The field holds the reader's lock until it is dropped.
    drop(field);
    // The file is complete only once the closing delimiter arrives; any
    // other field before it refuses the whole upload.
    if multipart
        .next_field()
        .await
        .map_err(multipart_error)?
        .is_some()
    {
        return Err(UploadError::Malformed);
    }
    drop(multipart);
    drain(body).await?;
    let staged = blocking(move || staging.finish())
        .await?
        .map_err(|e| failed(&e))?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel = CancelOnDrop(cancelled.clone());
    let objects = assets.library.objects.as_ref().expect("checked above");
    match ingest(
        &assets.database,
        objects,
        assets.library.registry.clone(),
        MANUAL_UPLOAD,
        &name,
        staged,
        cancelled,
    )
    .await
    {
        Ok(record) => Ok((
            StatusCode::CREATED,
            Json(json!({ "asset": entry(&record) })),
        )),
        Err(IngestError::Refused(_)) => Err(UploadError::Unsupported),
        Err(IngestError::EmptyName) => Err(UploadError::Malformed),
        Err(error) => Err(failed(&error)),
    }
}

/// Write `bytes` to staging on a blocking worker.
async fn write(
    mut staging: Staging,
    bytes: Vec<u8>,
    failed: &impl Fn(&dyn std::fmt::Display) -> UploadError,
) -> Result<Staging, UploadError> {
    let (staging, written) = blocking(move || {
        let written = staging.write(&bytes);
        (staging, written)
    })
    .await?;
    match written {
        Ok(()) => Ok(staging),
        Err(StageError::TooLarge) => Err(UploadError::TooLarge),
        Err(StageError::Storage(error)) => Err(failed(&error)),
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, UploadError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| UploadError::Failed)
}
