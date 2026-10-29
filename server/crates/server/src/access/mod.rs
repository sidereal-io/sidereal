//! The instance claim and the session routes.
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{any, get},
    Extension, Json, Router,
};
use serde_json::{json, Value};
use sidereal_core::{
    access::{ClaimError, Unavailable, User},
    database::Database,
};

pub mod cookie;
mod guard;

/// Every path under `/api`, behind the session guard. The catch-alls are
/// routes, not a fallback, because `route_layer` guards only matched routes:
/// an unknown path answers 401 before 404. `{*rest}` never matches an empty
/// segment, so `/api` and `/api/` need their own.
pub fn routes(database: Database) -> Router<Database> {
    Router::new()
        .route("/api/claim", get(claim_state).post(claim))
        .route(
            "/api/session",
            get(current_session).post(sign_in).delete(sign_out),
        )
        .route("/api", any(not_found))
        .route("/api/", any(not_found))
        .route("/api/{*rest}", any(not_found))
        .route_layer(middleware::from_fn_with_state(database, guard::session))
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
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

// The server listens on plain TCP only, so no request arrives over HTTPS
// yet. The trusted-proxy story (#450) sets this from the request.
const ARRIVED_OVER_HTTPS: bool = false;

async fn sign_in(
    State(database): State<Database>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, AccessError> {
    let credentials = Credentials::try_from(body)?;
    let (signed_in, token) = database
        .sign_in(&credentials.username, &credentials.password)
        .await?
        .ok_or(AccessError::WrongCredentials)?;
    let cookie = cookie::session(token.as_str(), ARRIVED_OVER_HTTPS);
    Ok(([(SET_COOKIE, cookie)], user(&signed_in)).into_response())
}

/// The guard has already looked the session up and refreshed the cookie.
async fn current_session(Extension(signed_in): Extension<User>) -> Json<Value> {
    user(&signed_in)
}

/// The guard has proven the token valid; it is still needed to delete the row.
async fn sign_out(
    State(database): State<Database>,
    headers: HeaderMap,
) -> Result<Response, AccessError> {
    let token = cookie::token(&headers).ok_or(AccessError::Unauthenticated)?;
    if !database.sign_out(token).await? {
        return Err(AccessError::Unauthenticated);
    }
    let cookie = cookie::cleared(ARRIVED_OVER_HTTPS);
    Ok((StatusCode::NO_CONTENT, [(SET_COOKIE, cookie)]).into_response())
}
