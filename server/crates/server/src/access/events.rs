//! The events socket: a WebSocket that lives only as long as its session.
use super::cookie;
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    http::HeaderMap,
    response::Response,
};
use sidereal_core::database::Database;
use std::time::Duration;

/// How often the socket beats, and how long the client may stay silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventTiming {
    pub heartbeat: Duration,
    pub silence: Duration,
}

impl Default for EventTiming {
    fn default() -> Self {
        Self {
            heartbeat: Duration::from_secs(30),
            silence: Duration::from_secs(60),
        }
    }
}

/// The guards have admitted the request, so the cookie names a live session.
pub async fn open(
    State((database, timing)): State<(Database, EventTiming)>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    let token = cookie::token(&headers).unwrap_or_default().to_owned();
    upgrade.on_upgrade(move |socket| watch(socket, database, token, timing))
}

async fn watch(_socket: WebSocket, _database: Database, _token: String, _timing: EventTiming) {}
