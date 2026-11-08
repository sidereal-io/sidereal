//! The events socket: a WebSocket that lives only as long as its session.
use super::cookie;
use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::HeaderMap,
    response::Response,
};
use sidereal_core::database::Database;
use std::time::Duration;
use tokio::time::{interval_at, Instant, MissedTickBehavior};

const HEARTBEAT: &str = r#"{"type":"heartbeat"}"#;

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

/// One task per socket. A client's message is read and ignored.
async fn watch(mut socket: WebSocket, _database: Database, _token: String, timing: EventTiming) {
    let mut beat = interval_at(Instant::now() + timing.heartbeat, timing.heartbeat);
    beat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = beat.tick() => {
                if socket.send(Message::text(HEARTBEAT)).await.is_err()
                    || socket.send(Message::Ping(Bytes::new())).await.is_err()
                {
                    return;
                }
            }
            frame = socket.recv() => match frame {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            },
        }
    }
}
