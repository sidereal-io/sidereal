//! The origin check and the session guard: one place decides who may call
//! `/api`, so a new route is protected without code of its own.
use super::{cookie, forwarded::Forwarded, AccessError};
use crate::config::AllowedOrigins;
use axum::{
    extract::{Request, State},
    http::{
        header::{HOST, ORIGIN, REFERER, SET_COOKIE, UPGRADE},
        HeaderMap, Method,
    },
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

/// Refuse a state-changing request that names an origin which is not
/// allowed. A request with neither `Origin` nor `Referer` passes: browsers
/// send one of them on a cross-site request, and other callers carry no
/// ambient cookie to abuse. This reads no database.
///
/// A WebSocket upgrade is a `GET`, but it opens a channel that carries the
/// session, so it is checked too: browsers do not apply CORS to it.
pub async fn origin(
    State(allowed): State<AllowedOrigins>,
    request: Request,
    next: Next,
) -> Response {
    if [Method::GET, Method::HEAD, Method::OPTIONS].contains(request.method())
        && !upgrades_to_websocket(request.headers())
    {
        return next.run(request).await;
    }
    let headers = request.headers();
    let Some(origin) = headers.get(ORIGIN).or_else(|| headers.get(REFERER)) else {
        return next.run(request).await;
    };
    // An HTTP/2 request carries its host in the URI, not in a `Host` header.
    let host = headers
        .get(HOST)
        .and_then(|host| host.to_str().ok())
        .or_else(|| {
            request
                .uri()
                .authority()
                .map(|authority| authority.as_str())
        });
    let admitted = origin
        .to_str()
        .is_ok_and(|origin| allowed.admit(origin, host));
    if !admitted {
        return AccessError::Forbidden.into_response();
    }
    next.run(request).await
}

fn upgrades_to_websocket(headers: &HeaderMap) -> bool {
    headers
        .get_all(UPGRADE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|protocol| protocol.trim().eq_ignore_ascii_case("websocket"))
}

/// Admit a request with an administrator session, hand its `User` to the
/// handler, and set the cookie again so the browser's copy lives as long as
/// the sliding expiry.
pub async fn session(
    State(database): State<Database>,
    mut request: Request,
    next: Next,
) -> Response {
    // Read now: the request is gone by the time the refresh is set.
    let https = request
        .extensions()
        .get::<Forwarded>()
        .is_some_and(|forwarded| forwarded.https);
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
            let cleared = cookie::cleared(https);
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
        let refreshed = cookie::session(&token, https);
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
