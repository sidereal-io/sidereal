//! The instance claim and the session routes.
use crate::config::AllowedOrigins;
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{
        header::{RETRY_AFTER, SET_COOKIE},
        HeaderMap, StatusCode,
    },
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
pub mod events;
pub mod forwarded;
mod guard;
pub mod limit;

use events::EventTiming;
use forwarded::Forwarded;
use limit::{Key, Limiter, Refusal};
use std::sync::Arc;

/// Every path under `/api`, behind the origin check and then the session
/// guard. The layer added last runs first. The catch-alls are
/// routes, not a fallback, because `route_layer` guards only matched routes:
/// an unknown path answers 401 before 404. `{*rest}` never matches an empty
/// segment, so `/api` and `/api/` need their own.
pub fn routes(
    database: Database,
    allowed_origins: AllowedOrigins,
    timing: EventTiming,
    limiter: Arc<Limiter>,
) -> Router<Database> {
    Router::new()
        .route("/api/claim", get(claim_state).post(claim))
        .route(
            "/api/session",
            get(current_session).post(sign_in).delete(sign_out),
        )
        .route(
            "/api/events",
            get(events::open).with_state((database.clone(), timing)),
        )
        .route("/api", any(not_found))
        .route("/api/", any(not_found))
        .route("/api/{*rest}", any(not_found))
        .route_layer(middleware::from_fn_with_state(database, guard::session))
        .route_layer(middleware::from_fn_with_state(
            allowed_origins,
            guard::origin,
        ))
        .layer(Extension(limiter))
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
    Forbidden,
    Unavailable,
    UntrustedProxy,
    TooManyAttempts { retry_after: u64 },
}

impl IntoResponse for AccessError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::TooManyAttempts { retry_after } => {
                let body = Json(json!({ "error": "too many attempts" }));
                let wait = [(RETRY_AFTER, retry_after.to_string())];
                return (StatusCode::TOO_MANY_REQUESTS, wait, body).into_response();
            }
            Self::BadRequest => (StatusCode::BAD_REQUEST, "invalid credentials"),
            Self::WrongCredentials => (StatusCode::UNAUTHORIZED, "invalid credentials"),
            Self::Claimed => (StatusCode::CONFLICT, "claimed"),
            Self::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            Self::UntrustedProxy => (
                StatusCode::BAD_REQUEST,
                "untrusted proxy: set SIDEREAL_TRUSTED_PROXIES",
            ),
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

/// Whether the request arrived over HTTPS. Without the trusted-proxy layer
/// there is no `Forwarded`, and the request is plain HTTP.
fn over_https(forwarded: Option<Extension<Forwarded>>) -> bool {
    forwarded.is_some_and(|Extension(forwarded)| forwarded.https)
}

/// A limited address is refused before its body is read as credentials, so
/// it costs no password check. The attempt is counted before its check
/// starts, so attempts at the same time count against each other.
async fn sign_in(
    State(database): State<Database>,
    Extension(limiter): Extension<Arc<Limiter>>,
    forwarded: Option<Extension<Forwarded>>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, AccessError> {
    let key = Key::of(forwarded.as_ref().map(|Extension(forwarded)| forwarded));
    if let Some(refusal) = limiter.refused(key) {
        return Err(limited(refusal));
    }
    let credentials = Credentials::try_from(body)?;
    let attempt = limiter.admit(key).map_err(limited)?;
    let signed_in = match database
        .sign_in(&credentials.username, &credentials.password)
        .await
    {
        Ok(signed_in) => signed_in,
        Err(unavailable) => {
            attempt.released();
            return Err(unavailable.into());
        }
    };
    let (signed_in, token) = signed_in.ok_or(AccessError::WrongCredentials)?;
    attempt.succeeded();
    let cookie = cookie::session(token.as_str(), over_https(forwarded));
    Ok(([(SET_COOKIE, cookie)], user(&signed_in)).into_response())
}

fn limited(refusal: Refusal) -> AccessError {
    AccessError::TooManyAttempts {
        retry_after: refusal.retry_after,
    }
}

/// The guard has already looked the session up and refreshed the cookie.
async fn current_session(Extension(signed_in): Extension<User>) -> Json<Value> {
    user(&signed_in)
}

/// The guard has proven the token valid; it is still needed to delete the row.
async fn sign_out(
    State(database): State<Database>,
    forwarded: Option<Extension<Forwarded>>,
    headers: HeaderMap,
) -> Result<Response, AccessError> {
    let token = cookie::token(&headers).ok_or(AccessError::Unauthenticated)?;
    let cleared = [(SET_COOKIE, cookie::cleared(over_https(forwarded)))];
    // The row can vanish after the guard's lookup, as when two sign-outs
    // race. The token is dead either way, so the browser forgets it.
    if !database.sign_out(token).await? {
        return Ok((cleared, AccessError::Unauthenticated).into_response());
    }
    Ok((StatusCode::NO_CONTENT, cleared).into_response())
}
