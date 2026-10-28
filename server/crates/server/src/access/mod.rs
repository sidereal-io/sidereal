//! The instance claim and the session routes.
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sidereal_core::{
    access::{ClaimError, Unavailable, User},
    database::Database,
};

pub mod cookie;

pub fn routes() -> Router<Database> {
    Router::new().route("/api/claim", get(claim_state).post(claim))
}

/// Every refusal has a fixed body, so no driver error, password or hash can
/// reach a response.
#[derive(Debug)]
pub enum AccessError {
    BadRequest,
    WrongCredentials,
    Claimed,
    Unauthenticated,
    Unavailable,
}

impl IntoResponse for AccessError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::BadRequest => (StatusCode::BAD_REQUEST, "invalid credentials"),
            Self::WrongCredentials => (StatusCode::UNAUTHORIZED, "invalid credentials"),
            Self::Claimed => (StatusCode::CONFLICT, "claimed"),
            Self::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        };
        (status, Json(json!({ "error": error }))).into_response()
    }
}

impl From<Unavailable> for AccessError {
    fn from(_: Unavailable) -> Self {
        Self::Unavailable
    }
}

impl From<ClaimError> for AccessError {
    fn from(error: ClaimError) -> Self {
        match error {
            ClaimError::Invalid => Self::BadRequest,
            ClaimError::Claimed => Self::Claimed,
            ClaimError::Unavailable => Self::Unavailable,
        }
    }
}

struct Credentials {
    username: String,
    password: String,
}

/// Only a JSON object with string `username` and `password` is credentials.
/// A derived deserializer would also take a positional array.
impl TryFrom<Result<Json<Value>, JsonRejection>> for Credentials {
    type Error = AccessError;

    fn try_from(body: Result<Json<Value>, JsonRejection>) -> Result<Self, AccessError> {
        let Ok(Json(Value::Object(mut body))) = body else {
            return Err(AccessError::BadRequest);
        };
        match (body.remove("username"), body.remove("password")) {
            (Some(Value::String(username)), Some(Value::String(password))) => {
                Ok(Self { username, password })
            }
            _ => Err(AccessError::BadRequest),
        }
    }
}

fn user(user: &User) -> Json<Value> {
    Json(json!({ "username": user.username, "role": user.role.as_str() }))
}

async fn claim_state(State(database): State<Database>) -> Result<Json<Value>, AccessError> {
    Ok(Json(json!({ "open": database.claim_open().await? })))
}

async fn claim(
    State(database): State<Database>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AccessError> {
    let credentials = Credentials::try_from(body)?;
    let created = database
        .claim(&credentials.username, &credentials.password)
        .await?;
    Ok((StatusCode::CREATED, user(&created)))
}
