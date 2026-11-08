//! The events socket, served on a real port: an upgrade needs a connection,
//! which `oneshot` cannot give it.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use futures_util::{SinkExt, StreamExt};
use sidereal_core::database::{initialize, Database};
use sidereal_server::{
    app_with_event_timing, config::AllowedOrigins, with_trusted_proxies, EventTiming,
};
use sqlx::{AssertSqlSafe, Connection, PgConnection};
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest, handshake::client::Response, protocol::frame::coding::CloseCode,
        Error, Message,
    },
    MaybeTlsStream, WebSocketStream,
};
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PASSWORD: &str = "correct-horse-battery";

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Short enough that a test waits well under a second for a tick. The
/// silence window is long, so a test that stops reading for a while, as
/// while a password is hashed, is not dropped for it.
const FAST: EventTiming = EventTiming {
    heartbeat: Duration::from_millis(200),
    silence: Duration::from_secs(10),
};

/// A silence window a test can wait out.
const QUIET: EventTiming = EventTiming {
    heartbeat: Duration::from_millis(200),
    silence: Duration::from_millis(600),
};

/// Long enough that a close within a second came from a client message,
/// not from a tick.
const SLOW: EventTiming = EventTiming {
    heartbeat: Duration::from_secs(10),
    silence: Duration::from_secs(20),
};

/// A claimed instance served on `127.0.0.1:0`, the cookie Sam got by
/// signing in, and the server's own database handle.
struct Served {
    addr: SocketAddr,
    cookie: String,
    database: Database,
}

async fn served(db: &TestDatabase, timing: EventTiming) -> Served {
    let database = initialize(
        db.options.clone(),
        Duration::from_secs(10),
        Duration::from_secs(10),
    )
    .await
    .unwrap();
    let app = with_trusted_proxies(
        app_with_event_timing(database.clone(), AllowedOrigins::default(), timing),
        Default::default(),
    );
    let credentials = serde_json::json!({ "username": "sam", "password": PASSWORD }).to_string();
    let claimed = post(&app, "/api/claim", &credentials).await;
    assert_eq!(claimed.status(), StatusCode::CREATED);
    let signed_in = post(&app, "/api/session", &credentials).await;
    assert_eq!(signed_in.status(), StatusCode::OK);
    let cookie = signed_in.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });
    Served {
        addr,
        cookie,
        database,
    }
}

async fn post(app: &Router, uri: &str, body: &str) -> axum::response::Response {
    let request = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

impl Served {
    async fn open(&self) -> Result<Socket, Error> {
        self.open_with(Some(&self.cookie)).await
    }

    async fn open_with(&self, cookie: Option<&str>) -> Result<Socket, Error> {
        self.handshake(cookie).await.map(|(socket, _)| socket)
    }

    async fn handshake(&self, cookie: Option<&str>) -> Result<(Socket, Response), Error> {
        let mut request = format!("ws://{}/api/events", self.addr)
            .into_client_request()
            .unwrap();
        if let Some(cookie) = cookie {
            request
                .headers_mut()
                .insert(header::COOKIE, cookie.parse().unwrap());
        }
        connect_async(request).await
    }
}

async fn execute(db: &TestDatabase, sql: &'static str) {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    sqlx::query(AssertSqlSafe(sql))
        .execute(&mut conn)
        .await
        .unwrap();
}

/// The refused upgrade's status and body.
fn refusal(result: Result<Socket, Error>) -> (StatusCode, String) {
    match result {
        Err(Error::Http(response)) => (
            response.status(),
            String::from_utf8(response.body().clone().unwrap_or_default()).unwrap(),
        ),
        Err(error) => panic!("not an HTTP refusal: {error}"),
        Ok(_) => panic!("the socket opened"),
    }
}

const UNAUTHENTICATED: &str = r#"{"error":"unauthenticated"}"#;

#[tokio::test]
async fn upgrade_with_a_session_opens_the_socket() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let (_, response) = served.handshake(Some(&served.cookie)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
}

#[tokio::test]
async fn upgrade_without_a_cookie_is_unauthenticated() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let (status, body) = refusal(served.open_with(None).await);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, UNAUTHENTICATED);
}

#[tokio::test]
async fn upgrade_with_a_revoked_session_is_unauthenticated() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    execute(&db, "DELETE FROM public.sessions").await;
    let (status, body) = refusal(served.open().await);
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, UNAUTHENTICATED);
}

#[tokio::test]
async fn upgrade_missing_from_a_plain_get_is_a_client_error() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let mut conn = TcpStream::connect(served.addr).await.unwrap();
    let request = format!(
        "GET /api/events HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nConnection: close\r\n\r\n",
        served.addr, served.cookie
    );
    tokio::io::AsyncWriteExt::write_all(&mut conn, request.as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut conn, &mut response)
        .await
        .unwrap();
    let status: u16 = response.split(' ').nth(1).unwrap().parse().unwrap();
    assert!((400..500).contains(&status), "{response}");
}

/// The next frame, or `None` when the connection ends first. Fails the test
/// if nothing arrives within `within`.
async fn next(socket: &mut Socket, within: Duration) -> Option<Message> {
    tokio::time::timeout(within, socket.next())
        .await
        .expect("no frame arrived in time")
        .and_then(Result::ok)
}

fn heartbeat() -> Message {
    Message::text(r#"{"type":"heartbeat"}"#)
}

#[tokio::test]
async fn heartbeat_text_and_a_ping_arrive_each_period() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let mut socket = served.open().await.unwrap();
    for _ in 0..2 {
        assert_eq!(
            next(&mut socket, FAST.heartbeat * 2).await,
            Some(heartbeat())
        );
        assert!(matches!(
            next(&mut socket, FAST.heartbeat).await,
            Some(Message::Ping(_))
        ));
    }
}

#[tokio::test]
async fn heartbeat_is_the_only_answer_to_a_client_message() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let mut socket = served.open().await.unwrap();
    socket.send(Message::text("hello")).await.unwrap();
    assert_eq!(
        next(&mut socket, FAST.heartbeat * 2).await,
        Some(heartbeat())
    );
}

/// Read until the server's close frame, which must come within `within`,
/// and answer its code. The client's reply is sent on the next read, which
/// also sees the connection end.
async fn close_code(socket: &mut Socket, within: Duration) -> CloseCode {
    let code = tokio::time::timeout(within, async {
        loop {
            match socket.next().await.and_then(Result::ok) {
                Some(Message::Close(Some(frame))) => return frame.code,
                Some(Message::Close(None)) => panic!("a close frame with no code"),
                Some(_) => {}
                None => panic!("the connection ended without a close frame"),
            }
        }
    })
    .await
    .expect("no close frame arrived in time");
    assert_eq!(next(socket, Duration::from_secs(1)).await, None);
    code
}

const PROMPTLY: Duration = Duration::from_secs(1);

#[tokio::test]
async fn recheck_on_a_message_after_revocation_closes_with_1008() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    execute(&db, "DELETE FROM public.sessions").await;
    socket.send(Message::text("anything")).await.unwrap();
    assert_eq!(close_code(&mut socket, PROMPTLY).await, CloseCode::Policy);
}

#[tokio::test]
async fn recheck_on_a_binary_message_after_revocation_closes_with_1008() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    execute(&db, "DELETE FROM public.sessions").await;
    socket.send(Message::binary(vec![1, 2, 3])).await.unwrap();
    assert_eq!(close_code(&mut socket, PROMPTLY).await, CloseCode::Policy);
}

#[tokio::test]
async fn recheck_on_a_tick_after_revocation_closes_a_silent_client_with_1008() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let mut socket = served.open().await.unwrap();
    execute(&db, "DELETE FROM public.sessions").await;
    assert_eq!(
        close_code(&mut socket, FAST.heartbeat * 2).await,
        CloseCode::Policy
    );
}

#[tokio::test]
async fn recheck_after_expiry_closes_with_1008() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    execute(
        &db,
        "UPDATE public.sessions SET expires_at = now() - interval '1 minute'",
    )
    .await;
    socket.send(Message::text("anything")).await.unwrap();
    assert_eq!(close_code(&mut socket, PROMPTLY).await, CloseCode::Policy);
}

#[tokio::test]
async fn recheck_after_access_recovery_closes_with_1008() {
    let db = TestDatabase::new();
    let served = served(&db, FAST).await;
    let mut socket = served.open().await.unwrap();
    served
        .database
        .reset_admin_password("another-horse-battery")
        .await
        .unwrap();
    assert_eq!(
        close_code(&mut socket, FAST.heartbeat * 2).await,
        CloseCode::Policy
    );
}

#[tokio::test]
async fn recheck_without_the_database_closes_with_1011() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    served.database.close().await;
    socket.send(Message::text("anything")).await.unwrap();
    assert_eq!(close_code(&mut socket, PROMPTLY).await, CloseCode::Error);
}

#[tokio::test]
async fn recheck_of_a_live_session_keeps_the_socket() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    socket.send(Message::text("anything")).await.unwrap();
    assert!(tokio::time::timeout(PROMPTLY, socket.next()).await.is_err());
}

#[tokio::test]
async fn recheck_is_not_a_use_of_the_session() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    execute(
        &db,
        "UPDATE public.sessions SET last_used_at = now() - interval '2 days', \
         expires_at = now() - interval '2 days' + interval '30 days'",
    )
    .await;
    let expiry = "SELECT expires_at::text FROM public.sessions";
    let before = scalar(&db, expiry).await;
    socket.send(Message::text("anything")).await.unwrap();
    assert!(tokio::time::timeout(PROMPTLY, socket.next()).await.is_err());
    assert_eq!(scalar(&db, expiry).await, before);
}

async fn scalar(db: &TestDatabase, sql: &'static str) -> String {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    sqlx::query_scalar(AssertSqlSafe(sql))
        .fetch_one(&mut conn)
        .await
        .unwrap()
}

/// Whether the connection ends within `within`, reading every frame on the
/// way. Reading answers the server's pings.
async fn ends_within(socket: &mut Socket, within: Duration) -> bool {
    tokio::time::timeout(within, async {
        while let Some(Ok(_)) = socket.next().await {}
    })
    .await
    .is_ok()
}

#[tokio::test]
async fn silent_client_is_dropped_after_the_silence_window() {
    let db = TestDatabase::new();
    let served = served(&db, QUIET).await;
    let mut socket = served.open().await.unwrap();
    // Not reading means the client answers no ping.
    tokio::time::sleep(QUIET.silence + QUIET.heartbeat).await;
    assert!(ends_within(&mut socket, QUIET.silence).await);
}

#[tokio::test]
async fn silent_client_that_answers_pings_stays_open() {
    let db = TestDatabase::new();
    let served = served(&db, QUIET).await;
    let mut socket = served.open().await.unwrap();
    assert!(!ends_within(&mut socket, QUIET.silence * 3).await);
}

#[tokio::test]
async fn closing_by_the_client_gets_the_server_reply() {
    let db = TestDatabase::new();
    let served = served(&db, SLOW).await;
    let mut socket = served.open().await.unwrap();
    socket.close(None).await.unwrap();
    assert!(matches!(
        next(&mut socket, PROMPTLY).await,
        Some(Message::Close(_))
    ));
}
