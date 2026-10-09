//! The HTTP shell exposes fixed liveness and readiness responses.
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::{json, Value};
use sidereal_core::database::Database;

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

async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
async fn readyz(State(database): State<Database>) -> (StatusCode, Json<Value>) {
    if database.ready().await {
        (StatusCode::OK, Json(json!({"status":"ready"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status":"not_ready"})),
        )
    }
}
