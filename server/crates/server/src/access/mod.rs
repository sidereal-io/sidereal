//! The instance claim and the session routes.
use crate::config::AllowedOrigins;
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{
        header::{RETRY_AFTER, SET_COOKIE, USER_AGENT},
        HeaderMap, StatusCode,
    },
    middleware,
    response::{IntoResponse, Response},
    routing::{any, get},
    Extension, Json, Router,
};
use serde_json::{json, Value};
use sidereal_core::{
    access::{
        ClaimError, Kind, Outcome, SecurityEvent, SecurityEventLines, Source, Unavailable, User,
    },
    database::Database,
};

pub mod cookie;
pub mod events;
pub mod forwarded;
mod guard;
pub mod limit;
pub mod sweep;

use events::EventTiming;
use forwarded::Forwarded;
use limit::{Key, Limiter, Refusal};
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;

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
    log: Log,
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
        .layer(Extension(log))
}

/// Where the server writes its log lines, such as one for each security
/// event. Its default is stderr; a test captures the lines instead.
#[derive(Clone)]
pub struct Log(Arc<dyn Fn(&str) + Send + Sync>);

impl Log {
    pub fn new(write: impl Fn(&str) + Send + Sync + 'static) -> Self {
        Self(Arc::new(write))
    }

    fn write(&self, event: &SecurityEvent) {
        self.line(&event.to_string());
    }

    /// Write a line that is not a security event.
    pub(crate) fn line(&self, line: &str) {
        (self.0)(line);
    }
}

impl Default for Log {
    fn default() -> Self {
        Self::new(|line| eprintln!("{line}"))
    }
}

/// Write each line `siderealctl` sends to `log`, so an event from the box
/// reaches the server's log. It listens before it returns, so no line sent
/// after that is missed. A listener that fails is rebuilt, and a line sent
/// while it is down is lost. The task ends when the database closes.
pub async fn relay(database: &Database, log: Log) -> Result<JoinHandle<()>, Unavailable> {
    let mut lines = database.security_event_lines().await?;
    let database = database.clone();
    Ok(tokio::spawn(async move {
        loop {
            while let Ok(line) = lines.next().await {
                (log.0)(&line);
            }
            // The listener pool holds one connection, which the old listener
            // must give back before a new one can take it.
            drop(lines);
            if database.is_closed() {
                return;
            }
            eprintln!("database unavailable: lost the listener for security events from siderealctl; reconnecting");
            match listen_again(&database).await {
                Some(again) => lines = again,
                None => return,
            }
        }
    }))
}

/// A new listener, retried with a doubling wait, or `None` once the database
/// closes.
async fn listen_again(database: &Database) -> Option<SecurityEventLines> {
    let mut wait = Duration::from_millis(250);
    loop {
        tokio::time::sleep(wait).await;
        if database.is_closed() {
            return None;
        }
        if let Ok(lines) = database.security_event_lines().await {
            return Some(lines);
        }
        wait = (wait * 2).min(Duration::from_secs(30));
    }
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
    Extension(log): Extension<Log>,
    forwarded: Option<Extension<Forwarded>>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), AccessError> {
    let credentials = Credentials::try_from(body)?;
    let claimed = database
        .claim(
            &credentials.username,
            &credentials.password,
            http(forwarded.as_ref(), &headers),
        )
        .await?;
    log.write(&claimed.event);
    Ok((StatusCode::CREATED, user(&claimed.user)))
}

fn event(kind: Kind, outcome: Outcome, source: Source, username: Option<String>) -> SecurityEvent {
    SecurityEvent {
        kind,
        outcome,
        source,
        username,
        sessions_ended: None,
        retry_after: None,
    }
}

/// Where a request came from, as its security event names it.
fn http(forwarded: Option<&Extension<Forwarded>>, headers: &HeaderMap) -> Source {
    Source::Http {
        client_address: forwarded.and_then(|Extension(forwarded)| forwarded.client),
        user_agent: user_agent(headers),
    }
}

/// The first `User-Agent` value as text, cut to 256 characters, so a caller
/// cannot grow the line.
fn user_agent(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(USER_AGENT)?;
    Some(
        String::from_utf8_lossy(value.as_bytes())
            .chars()
            .take(256)
            .collect(),
    )
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
    Extension(log): Extension<Log>,
    forwarded: Option<Extension<Forwarded>>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, AccessError> {
    let key = Key::of(forwarded.as_ref().map(|Extension(forwarded)| forwarded));
    let source = http(forwarded.as_ref(), &headers);
    if let Some(refusal) = limiter.refused(key) {
        return Err(limited(refusal, &log, source));
    }
    let credentials = Credentials::try_from(body)?;
    let attempt = match limiter.admit(key) {
        Ok(attempt) => attempt,
        Err(refusal) => return Err(limited(refusal, &log, source)),
    };
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
    let Some((signed_in, token)) = signed_in else {
        // Known and unknown usernames write the same line, so it reveals neither.
        log.write(&event(
            Kind::SignIn,
            Outcome::WrongCredentials,
            source,
            None,
        ));
        return Err(AccessError::WrongCredentials);
    };
    attempt.succeeded();
    log.write(&event(
        Kind::SignIn,
        Outcome::Success,
        source,
        Some(signed_in.username.clone()),
    ));
    let cookie = cookie::session(token.as_str(), over_https(forwarded));
    Ok(([(SET_COOKIE, cookie)], user(&signed_in)).into_response())
}

/// The first refusal of a run is a failed sign-in with outcome `limited`.
/// It names the request's own address, not its /64, and never the username:
/// the limiter never sees one.
fn limited(refusal: Refusal, log: &Log, source: Source) -> AccessError {
    if refusal.new {
        let limited = SecurityEvent {
            retry_after: Some(refusal.retry_after),
            ..event(Kind::SignIn, Outcome::Limited, source, None)
        };
        log.write(&limited);
    }
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
    Extension(signed_in): Extension<User>,
    Extension(log): Extension<Log>,
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
    log.write(&event(
        Kind::SignOut,
        Outcome::Success,
        http(forwarded.as_ref(), &headers),
        Some(signed_in.username),
    ));
    Ok((StatusCode::NO_CONTENT, cleared).into_response())
}
