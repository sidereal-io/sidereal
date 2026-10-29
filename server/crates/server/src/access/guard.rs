//! The session guard: one place decides who may call `/api`, so a new route
//! is protected without code of its own.
use super::{cookie, AccessError, ARRIVED_OVER_HTTPS};
use axum::{
    extract::{Request, State},
    http::{header::SET_COOKIE, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sidereal_core::{access::Role, database::Database};

/// The claim and sign-in routes, which a caller uses before it has a session.
/// `HEAD` is listed because axum answers it with the `GET` handler while the
/// request still says `HEAD`.
const PUBLIC: [(Method, &str); 4] = [
    (Method::GET, "/api/claim"),
    (Method::HEAD, "/api/claim"),
    (Method::POST, "/api/claim"),
    (Method::POST, "/api/session"),
];

/// Admit a request with an administrator session, hand its `User` to the
/// handler, and set the cookie again so the browser's copy lives as long as
/// the sliding expiry.
pub async fn session(
    State(database): State<Database>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if PUBLIC
        .iter()
        .any(|(method, public)| method == request.method() && *public == path)
    {
        return next.run(request).await;
    }
    let Some(token) = cookie::token(request.headers()) else {
        return AccessError::Unauthenticated.into_response();
    };
    let token = token.to_owned();
    let user = match database.session(&token).await {
        Ok(Some(user)) => user,
        // The browser holds a token no row backs: make it forget it.
        Ok(None) => {
            let cleared = cookie::cleared(ARRIVED_OVER_HTTPS);
            return ([(SET_COOKIE, cleared)], AccessError::Unauthenticated).into_response();
        }
        Err(unavailable) => return AccessError::from(unavailable).into_response(),
    };
    // Every role must be decided here before it can reach a handler.
    match user.role {
        Role::Admin => {}
    }
    request.extensions_mut().insert(user);
    let mut response = next.run(request).await;
    // Sign-out clears the cookie, and that must win.
    if !sets_session_cookie(&response) {
        let refreshed = cookie::session(&token, ARRIVED_OVER_HTTPS);
        if let Ok(refreshed) = refreshed.parse() {
            response.headers_mut().append(SET_COOKIE, refreshed);
        }
    }
    response
}

fn sets_session_cookie(response: &Response) -> bool {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|value| value.starts_with(&format!("{}=", cookie::NAME)))
}
