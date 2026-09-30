//! Sidereal v2 server: the axum HTTP shell.
//!
//! The router is built here rather than inline in `main` so integration tests
//! can exercise it without binding a socket. Later middleware (CORS/CSRF/auth,
//! ADR-007) attaches to this router via tower layers.

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

/// Build the application router.
pub fn app() -> Router {
    Router::new().route("/healthz", get(healthz))
}

/// `GET /healthz` → `200 {"status":"ok"}`.
///
/// The M0 liveness probe that proves the stack boots end to end; it also
/// doubles as the Docker healthcheck target (#233).
async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
