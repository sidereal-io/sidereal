//! The HTTP shell exposes fixed liveness and readiness responses, and the
//! built web shell when the operator names its folder.
use access::limit::Limiter;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use config::{AllowedOrigins, TrustedProxies};
use serde_json::{json, Value};
use sidereal_core::database::Database;
use std::{
    path::{Component, Path},
    sync::Arc,
};
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

pub mod access;
pub mod assets;
pub mod config;
pub mod preview;
pub mod preview_worker;
pub mod siderealctl;
pub mod storage;

pub use access::{events::EventTiming, limit::Clock, Log};
pub use assets::Library;

/// What a test may change about the production routes, so it need not wait
/// out real intervals.
#[derive(Clone)]
pub struct Tuning {
    /// The events socket's heartbeat and silence window.
    pub events: EventTiming,
    /// Where the sign-in limit reads the time.
    pub clock: Clock,
    /// Where the access routes write their `security event:` lines.
    pub log: Log,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            events: EventTiming::default(),
            clock: access::limit::system_clock(),
            log: Log::default(),
        }
    }
}

/// Liveness is independent of database availability.
pub fn app() -> Router {
    Router::new().route("/healthz", get(healthz))
}

/// Production routes are constructed only after database initialization.
/// `allowed_origins` are the origins beyond the request's own whose
/// state-changing `/api` requests are accepted.
pub fn app_with_database(database: Database, allowed_origins: AllowedOrigins) -> Router {
    app_with_tuning(database, allowed_origins, Tuning::default())
}

/// The production routes, tuned by the caller, with no managed storage.
/// Each call starts its own sign-in counts, as a new process does.
pub fn app_with_tuning(
    database: Database,
    allowed_origins: AllowedOrigins,
    tuning: Tuning,
) -> Router {
    app_with_library(database, allowed_origins, tuning, Library::new(None))
}

/// The production routes with the Asset library's object store and
/// Registry.
pub fn app_with_library(
    database: Database,
    allowed_origins: AllowedOrigins,
    tuning: Tuning,
    library: Library,
) -> Router {
    let limiter = Arc::new(Limiter::new(tuning.clock));
    let assets = assets::routes(database.clone(), library, tuning.log.clone());
    app().merge(
        Router::new()
            .route("/readyz", get(readyz))
            .merge(access::routes(
                database.clone(),
                allowed_origins,
                tuning.events,
                limiter,
                tuning.log,
                assets,
            ))
            .with_state(database),
    )
}

/// Serve the files under `web_root`, when there is one. Routes win over files.
///
/// `/` answers with `index.html`. Every other folder path is not found: no
/// redirect, no folder index, and no fallback to `index.html`. The file
/// services answer 405 to methods other than `GET` and `HEAD`.
///
/// An asset page address, `/assets/<id>` with a lowercase Asset id, also
/// answers with `index.html`, so the page's address survives a reload. Such
/// an address is reserved: a file at that path is never served, and a path
/// that becomes one only once the file service decodes and normalizes it is
/// not found.
pub fn with_web_root(router: Router, web_root: Option<&Path>) -> Router {
    let Some(web_root) = web_root else {
        return router;
    };
    let index = ServeFile::new(web_root.join("index.html"));
    let files = ServeDir::new(web_root)
        .append_index_html_on_directories(false)
        .redirect_to_trailing_slash(false);
    let shell = index.clone();
    router
        .route_service("/", index)
        .fallback(move |request: Request| async move {
            let path = request.uri().path();
            if is_asset_page(path) {
                return shell.oneshot(request).await.into_response();
            }
            if reaches_asset_page(path) {
                return StatusCode::NOT_FOUND.into_response();
            }
            files.oneshot(request).await.into_response()
        })
}

/// Whether the file service would look for the file at an asset page
/// address. Like `ServeDir`, this decodes the path, then drops empty and `.`
/// segments, so `/assets/%30b6…`, `/assets//0b6…` and `/./assets/0b6…` all
/// reach `assets/0b6…`.
fn reaches_asset_page(path: &str) -> bool {
    let Ok(decoded) = percent_encoding::percent_decode(path.as_bytes()).decode_utf8() else {
        return false;
    };
    let mut normalized = String::new();
    for component in Path::new(&*decoded).components() {
        match component {
            Component::Normal(segment) => {
                normalized.push('/');
                normalized.push_str(&segment.to_string_lossy());
            }
            Component::RootDir | Component::CurDir => {}
            // `ServeDir` refuses any other component itself.
            _ => return false,
        }
    }
    is_asset_page(&normalized)
}

/// Whether `path` is exactly `/assets/` and a lowercase Asset id: hexadecimal
/// digits in groups of 8, 4, 4, 4 and 12, joined by hyphens.
fn is_asset_page(path: &str) -> bool {
    let Some(id) = path.strip_prefix("/assets/") else {
        return false;
    };
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}

/// Resolve every request's scheme and client address, refusing forwarded
/// headers from a peer that is not a trusted proxy. Apply it last, so it
/// wraps every route and the file fallback.
pub fn with_trusted_proxies(router: Router, trusted: TrustedProxies) -> Router {
    router.layer(axum::middleware::from_fn_with_state(
        trusted,
        access::forwarded::layer,
    ))
}

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
async fn readyz(State(database): State<Database>) -> (StatusCode, Json<Value>) {
    let probe = database.probe().await;
    if let Some(change) = probe.change {
        eprintln!("{change}");
    }
    if probe.is_ready() {
        (StatusCode::OK, Json(json!({"status":"ready"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status":"not_ready"})),
        )
    }
}
