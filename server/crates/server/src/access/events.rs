//! The events socket: a WebSocket that lives only as long as its session.
use super::cookie;
use axum::{
    body::Bytes,
    extract::{
        ws::{close_code, CloseCode, CloseFrame, Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::HeaderMap,
    response::Response,
};
use sidereal_core::database::Database;
use std::time::Duration;
use tokio::time::{interval_at, sleep, timeout, Instant, MissedTickBehavior};

const HEARTBEAT: &str = r#"{"type":"heartbeat"}"#;
/// How long the server waits for the client to answer its close frame.
const CLOSING_GRACE: Duration = Duration::from_secs(5);

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

/// One task per socket. The session is checked again before each heartbeat
/// and on each client message, whose content is ignored. A client that sends
/// nothing, not even a pong, for the silence window is dropped.
async fn watch(mut socket: WebSocket, database: Database, token: String, timing: EventTiming) {
    let mut beat = interval_at(Instant::now() + timing.heartbeat, timing.heartbeat);
    beat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let silence = sleep(timing.silence);
    tokio::pin!(silence);
    loop {
        tokio::select! {
            () = &mut silence => return,
            _ = beat.tick() => {
                if let Some(code) = recheck(&database, &token).await {
                    return close(socket, code).await;
                }
                if socket.send(Message::text(HEARTBEAT)).await.is_err()
                    || socket.send(Message::Ping(Bytes::new())).await.is_err()
                {
                    return;
                }
            }
            frame = socket.recv() => match frame {
                Some(Ok(Message::Text(_) | Message::Binary(_))) => {
                    silence.as_mut().reset(Instant::now() + timing.silence);
                    if let Some(code) = recheck(&database, &token).await {
                        return close(socket, code).await;
                    }
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {
                    silence.as_mut().reset(Instant::now() + timing.silence);
                }
                // The reply to the client's close frame is queued, and goes
                // out on the next read. Without it a browser reports 1006.
                Some(Ok(Message::Close(_))) => {
                    let _ = timeout(CLOSING_GRACE, socket.recv()).await;
                    return;
                }
                Some(Err(_)) | None => return,
            },
        }
    }
}

/// The close code for a session that can no longer hold the socket, or
/// `None` while it can.
async fn recheck(database: &Database, token: &str) -> Option<CloseCode> {
    match database.session_alive(token).await {
        Ok(true) => None,
        Ok(false) => Some(close_code::POLICY),
        Err(_) => Some(close_code::ERROR),
    }
}

/// Send the close frame, then wait for the client's. Dropping the connection
/// with its reply unread would send a TCP reset, and the browser could then
/// report 1006 in place of `code`.
async fn close(mut socket: WebSocket, code: CloseCode) {
    let frame = CloseFrame {
        code,
        reason: "".into(),
    };
    if socket.send(Message::Close(Some(frame))).await.is_err() {
        return;
    }
    let _ = timeout(CLOSING_GRACE, async {
        while let Some(Ok(frame)) = socket.recv().await {
            if matches!(frame, Message::Close(_)) {
                return;
            }
        }
    })
    .await;
}
