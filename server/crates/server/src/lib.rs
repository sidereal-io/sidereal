//! The HTTP shell exposes fixed liveness and readiness responses, and the
//! built web shell when the operator names its folder.
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use sidereal_core::database::Database;
use std::path::Path;
use tower_http::services::{ServeDir, ServeFile};

pub mod config;

/// Liveness is independent of database availability.
pub fn app() -> Router {
    Router::new().route("/healthz", get(healthz))
}

/// Production routes are constructed only after database initialization.
pub fn app_with_database(database: Database) -> Router {
    app().merge(
        Router::new()
            .route("/readyz", get(readyz))
            .with_state(database),
    )
}

/// Serve the files under `web_root`, when there is one. Routes win over files.
///
/// `/` answers with `index.html`. Every other folder path is not found: no
/// redirect, no folder index, and no fallback to `index.html`. The file
/// services answer 405 to methods other than `GET` and `HEAD`.
pub fn with_web_root(router: Router, web_root: Option<&Path>) -> Router {
    let Some(web_root) = web_root else {
        return router;
    };
    let files = ServeDir::new(web_root)
        .append_index_html_on_directories(false)
        .redirect_to_trailing_slash(false);
    router
        .route_service("/", ServeFile::new(web_root.join("index.html")))
        .fallback_service(files)
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
