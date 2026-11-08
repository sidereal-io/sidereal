//! The events socket, served on a real port: an upgrade needs a connection,
//! which `oneshot` cannot give it.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    Router,
};
use sidereal_core::database::initialize;
use sidereal_server::{
    app_with_event_timing, config::AllowedOrigins, with_trusted_proxies, EventTiming,
};
use sqlx::{AssertSqlSafe, Connection, PgConnection};
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, handshake::client::Response, Error},
    MaybeTlsStream, WebSocketStream,
};
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PASSWORD: &str = "correct-horse-battery";

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Short enough that a test waits well under a second for a tick.
const FAST: EventTiming = EventTiming {
    heartbeat: Duration::from_millis(200),
    silence: Duration::from_millis(600),
};

/// A claimed instance served on `127.0.0.1:0`, and the cookie Sam got by
/// signing in.
struct Served {
    addr: SocketAddr,
    cookie: String,
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
        app_with_event_timing(database, AllowedOrigins::default(), timing),
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
    Served { addr, cookie }
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
