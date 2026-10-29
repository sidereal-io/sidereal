//! The claim and session routes, driven through the router against a fresh
//! database per test.
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use sidereal_core::database::{initialize, Database};
use sidereal_server::{app_with_database, config::AllowedOrigins};
use sqlx::{Connection, PgConnection};
use std::time::Duration;
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PASSWORD: &str = "correct-horse-battery";

async fn database(db: &TestDatabase) -> Database {
    initialize(
        db.options.clone(),
        Duration::from_secs(10),
        Duration::from_secs(10),
    )
    .await
    .unwrap()
}

async fn router(db: &TestDatabase) -> Router {
    app_with_database(database(db).await, AllowedOrigins::default())
}

fn credentials(username: &str, password: &str) -> String {
    serde_json::json!({ "username": username, "password": password }).to_string()
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<String>) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let body = body.map_or_else(Body::empty, Body::from);
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn body(response: Response) -> String {
    String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec()).unwrap()
}

async fn json(response: Response) -> serde_json::Value {
    serde_json::from_str(&body(response).await).unwrap()
}

fn sam() -> serde_json::Value {
    serde_json::json!({ "username": "sam", "role": "admin" })
}

async fn claim(app: &Router, username: &str, password: &str) -> Response {
    send(
        app,
        "POST",
        "/api/claim",
        Some(credentials(username, password)),
    )
    .await
}

async fn users(db: &TestDatabase) -> i64 {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    sqlx::query_scalar("SELECT count(*) FROM public.users")
        .fetch_one(&mut conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn claim_state_is_open_while_unclaimed() {
    let db = TestDatabase::new();
    let response = send(&router(&db).await, "GET", "/api/claim", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, r#"{"open":true}"#);
}

#[tokio::test]
async fn claim_state_is_closed_once_claimed() {
    let db = TestDatabase::new();
    let app = router(&db).await;
    claim(&app, "sam", PASSWORD).await;
    let response = send(&app, "GET", "/api/claim", None).await;
    assert_eq!(body(response).await, r#"{"open":false}"#);
}

#[tokio::test]
async fn claim_state_stays_closed_on_a_new_router() {
    let db = TestDatabase::new();
    claim(&router(&db).await, "sam", PASSWORD).await;
    let response = send(&router(&db).await, "GET", "/api/claim", None).await;
    assert_eq!(body(response).await, r#"{"open":false}"#);
}

#[tokio::test]
async fn claim_state_is_unavailable_when_the_pool_is_closed() {
    let db = TestDatabase::new();
    let database = database(&db).await;
    database.close().await;
    let response = send(
        &app_with_database(database, AllowedOrigins::default()),
        "GET",
        "/api/claim",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await, r#"{"error":"unavailable"}"#);
}

#[tokio::test]
async fn first_claim_creates_the_administrator_without_a_cookie() {
    let db = TestDatabase::new();
    let response = claim(&router(&db).await, "sam", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
    assert_eq!(json(response).await, sam());
    assert_eq!(users(&db).await, 1);
}

#[tokio::test]
async fn claim_with_bad_credentials_is_a_bad_request() {
    let db = TestDatabase::new();
    let app = router(&db).await;
    for (case, request) in [
        ("short password", Some(credentials("sam", "short"))),
        ("padded username", Some(credentials(" sam", PASSWORD))),
        ("array", Some(r#"["sam"]"#.into())),
        ("positional", Some(format!(r#"["sam","{PASSWORD}"]"#))),
        (
            "number",
            Some(r#"{"username":"sam","password":12345678}"#.into()),
        ),
        ("not JSON", Some("sam".into())),
        ("no body", None),
    ] {
        let response = send(&app, "POST", "/api/claim", request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{case}");
        assert_eq!(
            body(response).await,
            r#"{"error":"invalid credentials"}"#,
            "{case}"
        );
    }
    assert_eq!(users(&db).await, 0);
}

#[tokio::test]
async fn second_claim_is_a_conflict() {
    let db = TestDatabase::new();
    let app = router(&db).await;
    claim(&app, "sam", PASSWORD).await;
    let response = claim(&app, "alex", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(body(response).await, r#"{"error":"claimed"}"#);
    assert_eq!(users(&db).await, 1);
}

#[tokio::test]
async fn concurrent_claims_create_one_administrator() {
    let db = TestDatabase::new();
    let app = router(&db).await;
    let claims: Vec<_> = (0..10)
        .map(|n| {
            let app = app.clone();
            tokio::spawn(async move { claim(&app, &format!("user{n}"), PASSWORD).await.status() })
        })
        .collect();
    let mut statuses = Vec::new();
    for claim in claims {
        statuses.push(claim.await.unwrap());
    }
    let created = statuses
        .iter()
        .filter(|s| **s == StatusCode::CREATED)
        .count();
    let conflicts = statuses
        .iter()
        .filter(|s| **s == StatusCode::CONFLICT)
        .count();
    assert_eq!((created, conflicts), (1, 9));
    assert_eq!(users(&db).await, 1);
}

#[tokio::test]
async fn claim_is_unavailable_when_the_pool_is_closed() {
    let db = TestDatabase::new();
    let database = database(&db).await;
    database.close().await;
    let response = claim(
        &app_with_database(database, AllowedOrigins::default()),
        "sam",
        PASSWORD,
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await, r#"{"error":"unavailable"}"#);
    assert_eq!(users(&db).await, 0);
}

async fn claimed(db: &TestDatabase) -> Router {
    let app = router(db).await;
    assert_eq!(
        claim(&app, "sam", PASSWORD).await.status(),
        StatusCode::CREATED
    );
    app
}

async fn sign_in(app: &Router, username: &str, password: &str) -> Response {
    send(
        app,
        "POST",
        "/api/session",
        Some(credentials(username, password)),
    )
    .await
}

fn set_cookie(response: &Response) -> Option<String> {
    response
        .headers()
        .get(header::SET_COOKIE)
        .map(|value| value.to_str().unwrap().to_owned())
}

/// The `name=value` pair the browser sends back.
async fn signed_in(app: &Router) -> String {
    let response = sign_in(app, "sam", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::OK);
    set_cookie(&response)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn with_cookie(app: &Router, method: &str, cookie: &str) -> Response {
    let request = Request::builder()
        .method(method)
        .uri("/api/session")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(request).await.unwrap()
}

#[tokio::test]
async fn session_sign_in_names_the_user() {
    let db = TestDatabase::new();
    let response = sign_in(&claimed(&db).await, "sam", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await, sam());
}

#[tokio::test]
async fn session_sign_in_sets_the_session_cookie() {
    let db = TestDatabase::new();
    let response = sign_in(&claimed(&db).await, "sam", PASSWORD).await;
    let cookie = set_cookie(&response).unwrap();
    assert!(cookie.starts_with("sidereal_session="));
    assert!(cookie.len() > "sidereal_session=".len());
    for attribute in ["HttpOnly", "SameSite=Lax", "Path=/", "Max-Age=2592000"] {
        assert!(cookie.split("; ").any(|part| part == attribute), "{cookie}");
    }
    assert!(!cookie.contains("Secure"));
}

#[tokio::test]
async fn session_refusals_do_not_reveal_whether_the_username_exists() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let mut answers = Vec::new();
    for (username, password) in [("sam", "wrong-password"), ("nobody", PASSWORD)] {
        let response = sign_in(&app, username, password).await;
        let headers = format!("{:?}", response.headers());
        answers.push((response.status(), headers, body(response).await));
    }
    assert_eq!(answers[0].0, StatusCode::UNAUTHORIZED);
    assert_eq!(answers[0].2, r#"{"error":"invalid credentials"}"#);
    assert_eq!(answers[0], answers[1]);
}

#[tokio::test]
async fn session_sign_in_with_a_malformed_body_is_a_bad_request() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    for request in [Some(r#"["sam"]"#.into()), Some("sam".into()), None] {
        let response = send(&app, "POST", "/api/session", request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(body(response).await, r#"{"error":"invalid credentials"}"#);
    }
}

#[tokio::test]
async fn session_cookie_names_the_signed_in_user() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    let response = with_cookie(&app, "GET", &format!("theme=dark; {cookie}")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await, sam());
}

#[tokio::test]
async fn session_without_a_cookie_is_unauthenticated() {
    let db = TestDatabase::new();
    let response = send(&claimed(&db).await, "GET", "/api/session", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(response).await, r#"{"error":"unauthenticated"}"#);
}

#[tokio::test]
async fn session_with_an_unknown_token_is_unauthenticated() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    signed_in(&app).await;
    let response = with_cookie(&app, "GET", "sidereal_session=not-a-token").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(response).await, r#"{"error":"unauthenticated"}"#);
}

#[tokio::test]
async fn session_sign_out_clears_the_cookie() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    let response = with_cookie(&app, "DELETE", &cookie).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cleared = set_cookie(&response).unwrap();
    assert!(cleared.starts_with("sidereal_session=;"), "{cleared}");
    assert!(cleared.split("; ").any(|part| part == "Max-Age=0"));
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM public.sessions")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn session_cookie_is_refused_after_sign_out() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    with_cookie(&app, "DELETE", &cookie).await;
    let response = with_cookie(&app, "GET", &cookie).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn session_sign_out_without_a_session_is_unauthenticated() {
    let db = TestDatabase::new();
    let response = send(&claimed(&db).await, "DELETE", "/api/session", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(response).await, r#"{"error":"unauthenticated"}"#);
}

/// A running server binary, killed when the test ends.
mod spawned {
    use super::TestDatabase;
    use sqlx::ConnectOptions;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };

    pub struct Server {
        child: Child,
        pub port: u16,
    }

    impl Server {
        /// Start the server and wait until `/readyz` answers ready.
        pub fn ready(db: &TestDatabase) -> Self {
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let child = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
                .env("DATABASE_URL", db.options.to_url_lossy().to_string())
                .env("PORT", port.to_string())
                .env("DATABASE_STARTUP_TIMEOUT_SECONDS", "10")
                .env_remove("WEB_ROOT")
                .env_remove("STORAGE_ROOT")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut server = Self { child, port };
            let deadline = Instant::now() + Duration::from_secs(20);
            while !server
                .request("GET", "/readyz", None)
                .is_some_and(|r| r.ends_with(r#"{"status":"ready"}"#))
            {
                if let Some(status) = server.child.try_wait().unwrap() {
                    panic!("server exited before readiness: {status}");
                }
                assert!(Instant::now() < deadline, "server never became ready");
                std::thread::sleep(Duration::from_millis(100));
            }
            server
        }

        /// The whole HTTP response, head and body.
        pub fn request(&self, method: &str, path: &str, body: Option<&str>) -> Option<String> {
            let mut stream = TcpStream::connect(("127.0.0.1", self.port)).ok()?;
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
            let body = body.unwrap_or("");
            write!(
                stream,
                "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .ok()?;
            let mut response = String::new();
            stream.read_to_string(&mut response).ok()?;
            Some(response)
        }

        /// Stop the server and return everything it wrote to stdout and stderr.
        pub fn output(mut self) -> String {
            let _ = self.child.kill();
            let output = std::mem::replace(&mut self.child, Command::new("true").spawn().unwrap())
                .wait_with_output()
                .unwrap();
            format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

mod lifetime {
    use super::*;

    async fn moved(db: &TestDatabase, sql: &'static str) {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query(sql).execute(&mut conn).await.unwrap();
    }

    async fn holds(db: &TestDatabase, sql: &'static str) -> bool {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_scalar(sql).fetch_one(&mut conn).await.unwrap()
    }

    #[tokio::test]
    async fn an_expired_session_is_refused_and_deleted() {
        let db = TestDatabase::new();
        let app = claimed(&db).await;
        let cookie = signed_in(&app).await;
        moved(
            &db,
            "UPDATE public.sessions SET expires_at = now() - interval '1 minute'",
        )
        .await;
        let response = with_cookie(&app, "GET", &cookie).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(holds(&db, "SELECT NOT EXISTS (SELECT 1 FROM public.sessions)").await);
    }

    #[tokio::test]
    async fn a_use_moves_the_expiry_30_days_out() {
        let db = TestDatabase::new();
        let app = claimed(&db).await;
        let cookie = signed_in(&app).await;
        moved(&db, "UPDATE public.sessions SET last_used_at = now() - interval '29 days', expires_at = now() + interval '1 day'").await;
        let response = with_cookie(&app, "GET", &cookie).await;
        assert_eq!(response.status(), StatusCode::OK);
        let renewed = response.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(renewed.starts_with(&format!("{cookie};")));
        assert!(renewed.contains("Max-Age=2592000"));
        assert!(holds(&db, "SELECT bool_and(expires_at >= now() + interval '30 days' - interval '1 minute') FROM public.sessions").await);
    }

    #[tokio::test]
    async fn a_cookie_stays_valid_on_a_new_router() {
        let db = TestDatabase::new();
        let cookie = signed_in(&claimed(&db).await).await;
        let response = with_cookie(&router(&db).await, "GET", &cookie).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json(response).await, sam());
    }

    #[tokio::test]
    async fn no_answer_or_output_holds_the_password_or_its_hash() {
        const DISTINCTIVE: &str = "distinctive-claim-password-7f3a";
        let db = TestDatabase::new();
        let server = spawned::Server::ready(&db);
        let body = credentials("sam", DISTINCTIVE);
        let mut answers = String::new();
        for (method, path, body) in [
            ("POST", "/api/claim", Some(body.as_str())),
            ("POST", "/api/claim", Some(body.as_str())),
            ("POST", "/api/session", Some(body.as_str())),
            (
                "POST",
                "/api/session",
                Some(r#"{"username":"sam","password":"wrong-password"}"#),
            ),
        ] {
            answers += &server.request(method, path, body).unwrap();
        }
        assert!(answers.contains("201 Created") && answers.contains("200 OK"));
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        let hash: String = sqlx::query_scalar("SELECT password_hash FROM public.users")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        let captured = answers + &server.output();
        assert!(!captured.contains(DISTINCTIVE));
        assert!(!captured.contains(&hash));
    }
}

mod warning {
    use super::*;

    fn warnings(output: &str) -> usize {
        output
            .lines()
            .filter(|line| line.starts_with("instance unclaimed:"))
            .count()
    }

    #[tokio::test]
    async fn an_unclaimed_start_warns_once() {
        let db = TestDatabase::new();
        let output = spawned::Server::ready(&db).output();
        assert_eq!(warnings(&output), 1, "{output}");
    }

    #[tokio::test]
    async fn a_claimed_start_does_not_warn() {
        let db = TestDatabase::new();
        database(&db).await.claim("sam", PASSWORD).await.unwrap();
        let output = spawned::Server::ready(&db).output();
        assert_eq!(warnings(&output), 0, "{output}");
    }
}
