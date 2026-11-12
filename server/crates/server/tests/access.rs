//! The claim and session routes, driven through the router against a fresh
//! database per test.
use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use sidereal_core::database::{initialize, Database};
use sidereal_server::{
    app_with_database,
    config::{allowed_origins, trusted_proxies, AllowedOrigins, TrustedProxies},
    with_trusted_proxies,
};
use sqlx::{Connection, PgConnection};
use std::{net::SocketAddr, time::Duration};
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

mod common;
use common::WebRoot;

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

/// The production stack with no trusted proxies.
async fn router(db: &TestDatabase) -> Router {
    trusting(db, TrustedProxies::default()).await
}

async fn trusting(db: &TestDatabase, trusted: TrustedProxies) -> Router {
    with_trusted_proxies(
        app_with_database(database(db).await, AllowedOrigins::default()),
        trusted,
    )
}

fn proxies(value: &str) -> TrustedProxies {
    trusted_proxies(Some(std::ffi::OsStr::new(value))).unwrap()
}

/// A LAN client that is not a trusted proxy.
const PEER: &str = "192.168.1.20:40000";

fn credentials(username: &str, password: &str) -> String {
    serde_json::json!({ "username": username, "password": password }).to_string()
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<String>) -> Response {
    send_with(app, method, uri, body, &[]).await
}

async fn send_with(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<String>,
    headers: &[(&str, &str)],
) -> Response {
    send_from(app, PEER, method, uri, body, headers).await
}

/// A request from the TCP peer `peer`, as `into_make_service_with_connect_info`
/// reports it.
async fn send_from(
    app: &Router,
    peer: &str,
    method: &str,
    uri: &str,
    body: Option<String>,
    headers: &[(&str, &str)],
) -> Response {
    let peer: SocketAddr = peer.parse().unwrap();
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .extension(ConnectInfo(peer));
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
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
    // Two addresses, so the second refusal is not the sign-in limit's.
    for (peer, username, password) in [
        (PEER, "sam", "wrong-password"),
        ("192.168.1.21:40000", "nobody", PASSWORD),
    ] {
        let request = Some(credentials(username, password));
        let response = send_from(&app, peer, "POST", "/api/session", request, &[]).await;
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

const UNAUTHENTICATED: &str = r#"{"error":"unauthenticated"}"#;

#[tokio::test]
async fn guard_refuses_a_guarded_route_without_a_cookie() {
    let db = TestDatabase::new();
    let response = send(&claimed(&db).await, "DELETE", "/api/session", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(response).await, UNAUTHENTICATED);
}

#[tokio::test]
async fn guard_refuses_an_unknown_api_path_without_a_cookie() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    for (method, uri) in [
        ("GET", "/api/does-not-exist"),
        ("POST", "/api/does-not-exist/deeper"),
        ("GET", "/api/"),
        ("GET", "/api"),
    ] {
        let response = send(&app, method, uri, None).await;
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
        assert_eq!(body(response).await, UNAUTHENTICATED, "{method} {uri}");
    }
}

#[tokio::test]
async fn guard_refuses_an_unsupported_method_on_a_public_path() {
    let db = TestDatabase::new();
    let response = send(&claimed(&db).await, "PUT", "/api/claim", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(response).await, UNAUTHENTICATED);
}

#[tokio::test]
async fn guard_lets_the_public_routes_through_without_a_cookie() {
    let db = TestDatabase::new();
    let app = router(&db).await;
    let statuses = [
        send(&app, "GET", "/api/claim", None).await.status(),
        send(&app, "HEAD", "/api/claim", None).await.status(),
        claim(&app, "sam", PASSWORD).await.status(),
        sign_in(&app, "sam", PASSWORD).await.status(),
    ];
    assert_eq!(
        statuses,
        [
            StatusCode::OK,
            StatusCode::OK,
            StatusCode::CREATED,
            StatusCode::OK
        ]
    );
}

#[tokio::test]
async fn guard_leaves_the_probes_and_web_shell_public() {
    let db = TestDatabase::new();
    let root = WebRoot::new("guard");
    let app = sidereal_server::with_web_root(router(&db).await, Some(&root.path));
    for uri in ["/healthz", "/readyz", "/"] {
        let response = send(&app, "GET", uri, None).await;
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }
}

fn assert_cleared(response: &Response) {
    let cleared = set_cookie(response).expect("a cleared cookie");
    assert!(cleared.starts_with("sidereal_session=;"), "{cleared}");
    assert!(
        cleared.split("; ").any(|part| part == "Max-Age=0"),
        "{cleared}"
    );
}

async fn execute(db: &TestDatabase, sql: &'static str) {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    sqlx::query(sql).execute(&mut conn).await.unwrap();
}

#[tokio::test]
async fn stale_cookie_with_an_unknown_token_is_cleared() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let response = with_cookie(&app, "GET", "sidereal_session=not-a-token").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_cleared(&response);
}

#[tokio::test]
async fn stale_cookie_of_an_expired_session_is_cleared() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    execute(
        &db,
        "UPDATE public.sessions SET expires_at = now() - interval '1 minute'",
    )
    .await;
    let response = with_cookie(&app, "GET", &cookie).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_cleared(&response);
}

#[tokio::test]
async fn stale_cookie_of_a_revoked_session_is_refused() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    execute(&db, "DELETE FROM public.sessions").await;
    let response = with_cookie(&app, "GET", &cookie).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_cleared(&response);
    assert_eq!(body(response).await, UNAUTHENTICATED);
}

#[tokio::test]
async fn stale_cookie_clearing_needs_a_cookie() {
    let db = TestDatabase::new();
    let response = send(&claimed(&db).await, "GET", "/api/session", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(response.headers().get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn session_refreshes_the_cookie_on_any_guarded_route() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    let request = Request::builder()
        .uri("/api/does-not-exist")
        .header(header::COOKIE, &cookie)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let renewed = set_cookie(&response).expect("a refreshed cookie");
    assert!(renewed.starts_with(&format!("{cookie};")), "{renewed}");
    assert!(renewed.split("; ").any(|part| part == "Max-Age=2592000"));
}

#[tokio::test]
async fn session_sign_out_sets_only_the_cleared_cookie() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    let response = with_cookie(&app, "DELETE", &cookie).await;
    let cookies: Vec<_> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .collect();
    assert_eq!(cookies.len(), 1, "{cookies:?}");
    assert_cleared(&response);
}

#[tokio::test]
async fn session_sign_out_that_finds_no_row_clears_the_cookie() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let cookie = signed_in(&app).await;
    // The row vanishes between the guard's lookup and the delete, as when
    // two sign-outs race: the delete finds nothing to remove.
    execute(
        &db,
        "CREATE FUNCTION public.keep() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RETURN NULL; END'",
    )
    .await;
    execute(
        &db,
        "CREATE TRIGGER keep BEFORE DELETE ON public.sessions FOR EACH ROW EXECUTE FUNCTION public.keep()",
    )
    .await;
    let response = with_cookie(&app, "DELETE", &cookie).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let cookies: Vec<_> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .collect();
    assert_eq!(cookies.len(), 1, "{cookies:?}");
    assert_cleared(&response);
}

#[tokio::test]
async fn session_token_is_returned_only_to_its_own_browser() {
    let db = TestDatabase::new();
    let app = claimed(&db).await;
    let signed = sign_in(&app, "sam", PASSWORD).await;
    let cookie = set_cookie(&signed).unwrap();
    let pair = cookie.split(';').next().unwrap();
    let used = with_cookie(&app, "GET", pair).await;
    let reused = set_cookie(&used).unwrap();
    assert_eq!(reused.split(';').next(), Some(pair));
    let token = pair.trim_start_matches("sidereal_session=");
    let stranger = send(&app, "GET", "/api/session", None).await;
    assert!(stranger.headers().get(header::SET_COOKIE).is_none());
    assert!(!body(stranger).await.contains(token));
}

mod origin {
    use super::*;

    const FORBIDDEN: &str = r#"{"error":"forbidden"}"#;

    async fn sign_in_with(app: &Router, headers: &[(&str, &str)]) -> Response {
        let body = Some(credentials("sam", PASSWORD));
        send_with(app, "POST", "/api/session", body, headers).await
    }

    async fn sessions(db: &TestDatabase) -> i64 {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_scalar("SELECT count(*) FROM public.sessions")
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    async fn listing(db: &TestDatabase, value: &str) -> Router {
        let listed = allowed_origins(Some(std::ffi::OsStr::new(value))).unwrap();
        let app = app_with_database(database(db).await, listed);
        assert_eq!(
            claim(&app, "sam", PASSWORD).await.status(),
            StatusCode::CREATED
        );
        app
    }

    #[tokio::test]
    async fn origin_of_a_same_site_sign_in_is_allowed() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://localhost:5000"),
        ];
        let response = sign_in_with(&claimed(&db).await, &headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn origin_with_a_default_port_matches_a_host_without_one() {
        let db = TestDatabase::new();
        let app = claimed(&db).await;
        for headers in [
            [
                ("host", "photos.example"),
                ("origin", "https://photos.example:443"),
            ],
            [
                ("host", "photos.example:443"),
                ("origin", "https://photos.example"),
            ],
        ] {
            let response = sign_in_with(&app, &headers).await;
            assert_eq!(response.status(), StatusCode::OK, "{headers:?}");
        }
    }

    #[tokio::test]
    async fn origin_of_a_cross_site_sign_in_is_forbidden() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://evil.example"),
        ];
        let response = sign_in_with(&claimed(&db).await, &headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
        assert_eq!(body(response).await, FORBIDDEN);
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn origin_of_a_cross_site_sign_out_is_forbidden() {
        let db = TestDatabase::new();
        let app = claimed(&db).await;
        let cookie = signed_in(&app).await;
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://evil.example"),
            ("cookie", cookie.as_str()),
        ];
        let response = send_with(&app, "DELETE", "/api/session", None, &headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(sessions(&db).await, 1);
    }

    #[tokio::test]
    async fn origin_from_a_matching_referer_is_allowed() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("referer", "http://localhost:5000/sign-in?next=/"),
        ];
        let response = sign_in_with(&claimed(&db).await, &headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn origin_from_a_mismatched_referer_is_forbidden() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("referer", "http://evil.example/"),
        ];
        let response = sign_in_with(&claimed(&db).await, &headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn origin_is_not_needed_without_either_header() {
        let db = TestDatabase::new();
        let headers = [("host", "localhost:5000")];
        let response = sign_in_with(&claimed(&db).await, &headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn origin_of_a_read_is_not_checked() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://evil.example"),
        ];
        let response = send_with(&router(&db).await, "GET", "/api/claim", None, &headers).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
    }

    #[tokio::test]
    async fn origin_before_session_refuses_without_reading_the_database() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database.close().await;
        let app = app_with_database(database, AllowedOrigins::default());
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://evil.example"),
            ("cookie", "sidereal_session=not-a-token"),
        ];
        let response = send_with(&app, "DELETE", "/api/session", None, &headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body(response).await, FORBIDDEN);
    }

    /// A WebSocket upgrade, which is a `GET`, with a database that cannot
    /// be read: a 403 proves the origin check refused it first, and a 503
    /// proves it passed on to the session guard.
    async fn upgrade_with(db: &TestDatabase, upgrade: &str, headers: &[(&str, &str)]) -> Response {
        let database = database(db).await;
        database.close().await;
        let app = app_with_database(database, AllowedOrigins::default());
        let mut headers = headers.to_vec();
        headers.extend([
            ("connection", "upgrade"),
            ("upgrade", upgrade),
            ("sec-websocket-version", "13"),
            ("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="),
            ("cookie", "sidereal_session=not-a-token"),
        ]);
        send_with(&app, "GET", "/api/events", None, &headers).await
    }

    #[tokio::test]
    async fn origin_of_a_cross_site_upgrade_is_forbidden_before_the_session_check() {
        let db = TestDatabase::new();
        for upgrade in ["websocket", "WebSocket", "h2c, websocket"] {
            let headers = [
                ("host", "localhost:5000"),
                ("origin", "http://evil.example"),
            ];
            let response = upgrade_with(&db, upgrade, &headers).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{upgrade}");
            assert_eq!(body(response).await, FORBIDDEN);
        }
    }

    #[tokio::test]
    async fn origin_of_an_upgrade_from_a_mismatched_referer_is_forbidden() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("referer", "http://evil.example/"),
        ];
        let response = upgrade_with(&db, "websocket", &headers).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn origin_of_a_same_site_upgrade_reaches_the_session_check() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://localhost:5000"),
        ];
        let response = upgrade_with(&db, "websocket", &headers).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn origin_is_not_needed_on_an_upgrade_without_either_header() {
        let db = TestDatabase::new();
        let headers = [("host", "localhost:5000")];
        let response = upgrade_with(&db, "websocket", &headers).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn origin_listed_in_the_environment_is_allowed_for_another_host() {
        let db = TestDatabase::new();
        let app = listing(&db, "https://photos.example, http://localhost:5173").await;
        for origin in ["https://photos.example", "http://localhost:5173"] {
            let headers = [("host", "app:5000"), ("origin", origin)];
            let response = sign_in_with(&app, &headers).await;
            assert_eq!(response.status(), StatusCode::OK, "{origin}");
        }
    }
}

mod untrusted_proxy {
    use super::*;

    const REFUSED: &str = r#"{"error":"untrusted proxy: set SIDEREAL_TRUSTED_PROXIES"}"#;

    async fn sessions(db: &TestDatabase) -> i64 {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_scalar("SELECT count(*) FROM public.sessions")
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    async fn assert_refused(response: Response) {
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
        assert_eq!(body(response).await, REFUSED);
    }

    #[tokio::test]
    async fn untrusted_proxy_sign_in_is_refused_without_a_session() {
        let db = TestDatabase::new();
        let app = claimed(&db).await;
        let body = Some(credentials("sam", PASSWORD));
        let headers = [("x-forwarded-proto", "https")];
        let response = send_with(&app, "POST", "/api/session", body, &headers).await;
        assert_refused(response).await;
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn untrusted_proxy_outside_the_list_is_refused_on_the_probe() {
        let db = TestDatabase::new();
        let app = trusting(&db, proxies("172.18.0.1")).await;
        let headers = [("x-forwarded-for", "203.0.113.9")];
        assert_refused(send_with(&app, "GET", "/healthz", None, &headers).await).await;
    }

    #[tokio::test]
    async fn untrusted_proxy_is_refused_on_the_web_shell() {
        let db = TestDatabase::new();
        let root = WebRoot::new("untrusted-proxy");
        let app = sidereal_server::with_web_root(
            app_with_database(database(&db).await, AllowedOrigins::default()),
            Some(&root.path),
        );
        let app = with_trusted_proxies(app, TrustedProxies::default());
        let headers = [("x-forwarded-for", "203.0.113.9")];
        for uri in ["/", "/assets/app.js", "/missing"] {
            assert_refused(send_with(&app, "GET", uri, None, &headers).await).await;
        }
    }

    #[tokio::test]
    async fn untrusted_proxy_is_refused_on_an_unknown_path() {
        let db = TestDatabase::new();
        let headers = [("x-forwarded-proto", "https")];
        let response = send_with(&router(&db).await, "GET", "/missing", None, &headers).await;
        assert_refused(response).await;
    }

    #[tokio::test]
    async fn untrusted_proxy_refusal_wins_over_the_origin_check() {
        let db = TestDatabase::new();
        let headers = [
            ("host", "localhost:5000"),
            ("origin", "http://evil.example"),
            ("x-forwarded-proto", "https"),
        ];
        let body = Some(credentials("sam", PASSWORD));
        let response = send_with(&claimed(&db).await, "POST", "/api/session", body, &headers).await;
        assert_refused(response).await;
    }

    #[tokio::test]
    async fn untrusted_proxy_refusal_reads_no_database() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        database.close().await;
        let app = with_trusted_proxies(
            app_with_database(database, AllowedOrigins::default()),
            TrustedProxies::default(),
        );
        let headers = [
            ("x-forwarded-proto", "https"),
            ("cookie", "sidereal_session=not-a-token"),
        ];
        assert_refused(send_with(&app, "GET", "/api/session", None, &headers).await).await;
    }

    #[tokio::test]
    async fn untrusted_proxy_without_a_peer_is_refused_whatever_the_list() {
        let db = TestDatabase::new();
        let app = trusting(&db, proxies("0.0.0.0/0, ::/0")).await;
        let request = Request::builder()
            .uri("/healthz")
            .header("x-forwarded-for", "203.0.113.9")
            .body(Body::empty())
            .unwrap();
        assert_refused(app.oneshot(request).await.unwrap()).await;
    }

    #[tokio::test]
    async fn untrusted_proxy_check_passes_a_request_without_forwarded_headers() {
        let db = TestDatabase::new();
        let response = send(&router(&db).await, "GET", "/healthz", None).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn untrusted_proxy_check_passes_a_listed_proxy() {
        let db = TestDatabase::new();
        let app = trusting(&db, proxies("172.18.0.1")).await;
        let headers = [("x-forwarded-for", "203.0.113.9")];
        let response = send_from(&app, "172.18.0.1:50000", "GET", "/healthz", None, &headers).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}

mod secure_cookie {
    use super::*;

    const PROXY: &str = "172.18.0.1:50000";

    async fn behind_proxy(db: &TestDatabase) -> Router {
        let app = trusting(db, proxies("172.18.0.1")).await;
        assert_eq!(
            claim(&app, "sam", PASSWORD).await.status(),
            StatusCode::CREATED
        );
        app
    }

    async fn proxied(
        app: &Router,
        method: &str,
        proto: &str,
        body: Option<String>,
        cookie: Option<&str>,
    ) -> Response {
        let mut headers = vec![("x-forwarded-proto", proto)];
        headers.extend(cookie.map(|cookie| ("cookie", cookie)));
        send_from(app, PROXY, method, "/api/session", body, &headers).await
    }

    fn is_secure(response: &Response) -> bool {
        let cookie = set_cookie(response).expect("a session cookie");
        cookie.split("; ").any(|part| part == "Secure")
    }

    #[tokio::test]
    async fn secure_cookie_after_sign_in_over_https() {
        let db = TestDatabase::new();
        let body = Some(credentials("sam", PASSWORD));
        let response = proxied(&behind_proxy(&db).await, "POST", "https", body, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = set_cookie(&response).unwrap();
        for attribute in ["HttpOnly", "SameSite=Lax", "Path=/", "Max-Age=2592000"] {
            assert!(cookie.split("; ").any(|part| part == attribute), "{cookie}");
        }
        assert!(is_secure(&response), "{cookie}");
    }

    #[tokio::test]
    async fn secure_cookie_is_not_set_over_plain_http() {
        let db = TestDatabase::new();
        let body = Some(credentials("sam", PASSWORD));
        let response = proxied(&behind_proxy(&db).await, "POST", "http", body, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!is_secure(&response));
    }

    #[tokio::test]
    async fn secure_cookie_on_the_refresh_of_a_guarded_request() {
        let db = TestDatabase::new();
        let app = behind_proxy(&db).await;
        let cookie = signed_in(&app).await;
        let response = proxied(&app, "GET", "https", None, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(is_secure(&response));
    }

    #[tokio::test]
    async fn secure_cookie_on_a_cleared_stale_cookie() {
        let db = TestDatabase::new();
        let app = behind_proxy(&db).await;
        let stale = "sidereal_session=not-a-token";
        let response = proxied(&app, "GET", "https", None, Some(stale)).await;
        assert_cleared(&response);
        assert!(is_secure(&response));
    }

    #[tokio::test]
    async fn secure_cookie_on_sign_out() {
        let db = TestDatabase::new();
        let app = behind_proxy(&db).await;
        let cookie = signed_in(&app).await;
        let response = proxied(&app, "DELETE", "https", None, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_cleared(&response);
        assert!(is_secure(&response));
    }
}

/// A running server binary, killed when the test ends.
mod spawned {
    use super::TestDatabase;
    use sqlx::ConnectOptions;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        process::{Child, Command, ExitStatus, Stdio},
        time::{Duration, Instant},
    };

    pub struct Server {
        child: Child,
        pub port: u16,
    }

    impl Server {
        /// Start the server and wait until `/readyz` answers ready.
        pub fn ready(db: &TestDatabase) -> Self {
            Self::ready_with(db, &[])
        }

        /// Start the server with extra environment variables and wait until
        /// `/readyz` answers ready.
        pub fn ready_with(db: &TestDatabase, env: &[(&str, &str)]) -> Self {
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
                .env_remove("SIDEREAL_ALLOWED_ORIGINS")
                .env_remove("SIDEREAL_TRUSTED_PROXIES")
                .envs(env.iter().copied())
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
            self.request_with(method, path, body, "localhost", &[])
        }

        /// The whole HTTP response to a request with this `Host` and these
        /// extra headers.
        pub fn request_with(
            &self,
            method: &str,
            path: &str,
            body: Option<&str>,
            host: &str,
            headers: &[(&str, &str)],
        ) -> Option<String> {
            let mut stream = TcpStream::connect(("127.0.0.1", self.port)).ok()?;
            stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
            let body = body.unwrap_or("");
            let extra: String = headers
                .iter()
                .map(|(name, value)| format!("{name}: {value}\r\n"))
                .collect();
            write!(
                stream,
                "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{extra}Connection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .ok()?;
            let mut response = String::new();
            stream.read_to_string(&mut response).ok()?;
            Some(response)
        }

        /// Send SIGTERM, as a container runtime does, and wait up to `within`
        /// for the server to exit. `None` means it was still running.
        pub fn terminate(&mut self, within: Duration) -> Option<ExitStatus> {
            let pid = libc::pid_t::try_from(self.child.id()).unwrap();
            // SAFETY: `kill` only sends a signal to the child this harness
            // spawned and has not yet waited on, so the pid is still its own.
            assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
            let deadline = Instant::now() + within;
            loop {
                if let Some(status) = self.child.try_wait().unwrap() {
                    return Some(status);
                }
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
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

mod allowed_origins_at_start {
    use super::*;
    use sqlx::ConnectOptions;
    use std::{
        process::{Command, Stdio},
        time::Instant,
    };

    #[tokio::test]
    async fn a_malformed_item_stops_the_server_before_the_database() {
        let db = TestDatabase::new();
        let mut child = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
            .env("DATABASE_URL", db.options.to_url_lossy().to_string())
            .env("PORT", "0")
            .env_remove("WEB_ROOT")
            .env_remove("STORAGE_ROOT")
            .env("SIDEREAL_ALLOWED_ORIGINS", "https://distinctive.example/")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() > deadline {
                child.kill().unwrap();
                panic!("the server kept running");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("SIDEREAL_ALLOWED_ORIGINS"), "{stderr}");
        assert!(!stderr.contains("distinctive"), "{stderr}");
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        let tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert_eq!(tables, 0);
    }

    #[tokio::test]
    async fn a_valid_list_admits_its_origins() {
        let db = TestDatabase::new();
        let server = spawned::Server::ready_with(
            &db,
            &[(
                "SIDEREAL_ALLOWED_ORIGINS",
                "https://photos.example, http://localhost:5173",
            )],
        );
        let body = credentials("sam", PASSWORD);
        let origin = [("Origin", "http://localhost:5173")];
        let answer = server
            .request_with("POST", "/api/claim", Some(&body), "app:5000", &origin)
            .unwrap();
        assert!(answer.starts_with("HTTP/1.1 201"), "{answer}");
    }
}

mod spawned_trusted_proxy {
    use super::*;
    use sqlx::ConnectOptions;
    use std::{
        process::{Command, Stdio},
        time::Instant,
    };

    #[tokio::test]
    async fn spawned_trusted_proxy_malformed_item_stops_the_server_before_the_database() {
        let db = TestDatabase::new();
        let mut child = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
            .env("DATABASE_URL", db.options.to_url_lossy().to_string())
            .env("PORT", "0")
            .env_remove("WEB_ROOT")
            .env_remove("STORAGE_ROOT")
            .env_remove("SIDEREAL_ALLOWED_ORIGINS")
            .env("SIDEREAL_TRUSTED_PROXIES", "proxy.example")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() > deadline {
                child.kill().unwrap();
                panic!("the server kept running");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("SIDEREAL_TRUSTED_PROXIES"), "{stderr}");
        assert!(!stderr.contains("proxy.example"), "{stderr}");
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        let tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert_eq!(tables, 0);
    }

    #[tokio::test]
    async fn spawned_trusted_proxy_over_https_gets_a_secure_cookie() {
        let db = TestDatabase::new();
        let server = spawned::Server::ready_with(&db, &[("SIDEREAL_TRUSTED_PROXIES", "127.0.0.1")]);
        let body = credentials("sam", PASSWORD);
        let claimed = server.request("POST", "/api/claim", Some(&body)).unwrap();
        assert!(claimed.starts_with("HTTP/1.1 201"), "{claimed}");
        let https = [("X-Forwarded-Proto", "https")];
        let answer = server
            .request_with("POST", "/api/session", Some(&body), "localhost", &https)
            .unwrap();
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        let cookie = answer
            .lines()
            .find(|line| line.to_ascii_lowercase().starts_with("set-cookie:"))
            .expect("a session cookie");
        assert!(
            cookie.trim_end().split("; ").any(|part| part == "Secure"),
            "{cookie}"
        );
    }

    #[tokio::test]
    async fn spawned_trusted_proxy_unset_refuses_forwarded_headers() {
        let db = TestDatabase::new();
        let server = spawned::Server::ready(&db);
        let https = [("X-Forwarded-Proto", "https")];
        let answer = server
            .request_with("GET", "/healthz", None, "localhost", &https)
            .unwrap();
        assert!(answer.starts_with("HTTP/1.1 400"), "{answer}");
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

mod spawned_events {
    use super::*;
    use tokio_tungstenite::tungstenite::{client::IntoClientRequest, http::header::COOKIE};

    /// The `name=value` cookie from signing in to a claimed spawned server.
    fn signed_in(server: &spawned::Server) -> String {
        let body = credentials("sam", PASSWORD);
        let claimed = server.request("POST", "/api/claim", Some(&body)).unwrap();
        assert!(claimed.starts_with("HTTP/1.1 201"), "{claimed}");
        let answer = server.request("POST", "/api/session", Some(&body)).unwrap();
        let cookie = answer
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("set-cookie").then_some(value)
            })
            .expect("a session cookie");
        cookie.trim().split(';').next().unwrap().to_owned()
    }

    #[tokio::test]
    async fn spawned_server_upgrades_and_exits_on_sigterm_with_a_socket_open() {
        let db = TestDatabase::new();
        let mut server = spawned::Server::ready(&db);
        let mut request = format!("ws://127.0.0.1:{}/api/events", server.port)
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert(COOKIE, signed_in(&server).parse().unwrap());
        let (socket, response) = tokio_tungstenite::connect_async(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
        let status = server.terminate(Duration::from_secs(10));
        drop(socket);
        assert!(
            status.is_some_and(|status| status.success()),
            "the server did not exit cleanly within 10 s: {status:?}"
        );
    }
}

mod sign_in_limit {
    use super::*;
    use sidereal_server::{app_with_tuning, Tuning};
    use std::{
        sync::{Arc, Mutex},
        time::Instant,
    };

    /// A clock that moves only when the test says so.
    pub struct Clock(Arc<Mutex<Instant>>);

    impl Clock {
        pub fn advance(&self, by: Duration) {
            *self.0.lock().unwrap() += by;
        }
    }

    pub fn seconds(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    /// The production stack on `database`, trusting `trusted`, with the sign-in
    /// limit on a manual clock.
    pub fn tuned(database: Database, trusted: TrustedProxies) -> (Router, Clock) {
        let now = Arc::new(Mutex::new(Instant::now()));
        let reading = now.clone();
        let tuning = Tuning {
            clock: Arc::new(move || *reading.lock().unwrap()),
            ..Tuning::default()
        };
        let app = app_with_tuning(database, AllowedOrigins::default(), tuning);
        (with_trusted_proxies(app, trusted), Clock(now))
    }

    /// A claimed instance trusting `trusted`, on a manual clock.
    pub async fn limited(db: &TestDatabase, trusted: TrustedProxies) -> (Router, Clock) {
        let (app, clock) = tuned(database(db).await, trusted);
        assert_eq!(
            claim(&app, "sam", PASSWORD).await.status(),
            StatusCode::CREATED
        );
        (app, clock)
    }

    pub async fn attempt(app: &Router, username: &str, password: &str) -> Response {
        sign_in(app, username, password).await
    }

    /// `count` wrong passwords for `username`, `apart` from each other,
    /// leaving the clock at the last one.
    pub async fn fail_as(app: &Router, clock: &Clock, usernames: &[&str], apart: Duration) {
        for (n, username) in usernames.iter().enumerate() {
            if n > 0 {
                clock.advance(apart);
            }
            let response = attempt(app, username, "wrong-password").await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "failure {n}");
        }
    }

    pub async fn fail(app: &Router, clock: &Clock, count: usize, apart: Duration) {
        fail_as(app, clock, &vec!["sam"; count], apart).await;
    }

    pub fn retry_after(response: &Response) -> Option<&str> {
        response
            .headers()
            .get(header::RETRY_AFTER)
            .map(|value| value.to_str().unwrap())
    }

    async fn sessions(db: &TestDatabase) -> i64 {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_scalar("SELECT count(*) FROM public.sessions")
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn sign_in_limit_refuses_correct_credentials_from_a_limited_address() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 1, Duration::ZERO).await;
        let response = attempt(&app, "sam", PASSWORD).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let wait: u64 = retry_after(&response).unwrap().parse().unwrap();
        assert!(wait >= 1);
        assert!(set_cookie(&response).is_none());
        assert_eq!(body(response).await, r#"{"error":"too many attempts"}"#);
        assert_eq!(sessions(&db).await, 0);
    }

    #[tokio::test]
    async fn sign_in_limit_refuses_a_malformed_body_from_a_limited_address() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 1, Duration::ZERO).await;
        let response = send(&app, "POST", "/api/session", Some("sam".into())).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body(response).await, r#"{"error":"too many attempts"}"#);
    }

    #[tokio::test]
    async fn sign_in_limit_refuses_before_reading_the_database() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        let (app, clock) = tuned(database.clone(), TrustedProxies::default());
        assert_eq!(
            claim(&app, "sam", PASSWORD).await.status(),
            StatusCode::CREATED
        );
        fail(&app, &clock, 1, Duration::ZERO).await;
        database.close().await;
        let response = attempt(&app, "sam", PASSWORD).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn sign_in_limit_refuses_a_second_attempt_within_the_second() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 1, Duration::ZERO).await;
        clock.advance(Duration::from_millis(999));
        let response = attempt(&app, "sam", PASSWORD).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(retry_after(&response), Some("1"));
    }

    #[tokio::test]
    async fn sign_in_limit_refuses_the_sixth_attempt_within_a_minute() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 5, seconds(2)).await;
        clock.advance(seconds(2));
        let response = attempt(&app, "sam", PASSWORD).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(retry_after(&response), Some("50"));
    }

    #[tokio::test]
    async fn sign_in_limit_admits_once_the_minute_tier_has_room() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 5, seconds(2)).await;
        clock.advance(seconds(61 - 8));
        assert_eq!(
            attempt(&app, "sam", PASSWORD).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn sign_in_limit_refuses_the_twenty_first_attempt_within_an_hour() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 20, seconds(15)).await;
        clock.advance(seconds(15));
        let response = attempt(&app, "sam", PASSWORD).await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(retry_after(&response), Some("3300"));
    }

    #[tokio::test]
    async fn sign_in_limit_counts_a_known_and_an_unknown_username_alike() {
        let mut answers = Vec::new();
        for usernames in [["sam", "nobody", "nobody", "nobody", "nobody"], ["sam"; 5]] {
            let db = TestDatabase::new();
            let (app, clock) = limited(&db, TrustedProxies::default()).await;
            fail_as(&app, &clock, &usernames, seconds(2)).await;
            clock.advance(seconds(2));
            let response = attempt(&app, "sam", PASSWORD).await;
            answers.push((response.status(), retry_after(&response).map(str::to_owned)));
        }
        assert_eq!(
            answers[0],
            (StatusCode::TOO_MANY_REQUESTS, Some("50".to_owned()))
        );
        assert_eq!(answers[0], answers[1]);
    }

    #[tokio::test]
    async fn sign_in_limit_refusals_do_not_extend_the_wait() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 5, seconds(2)).await;
        for _ in 0..40 {
            clock.advance(seconds(1));
            let response = attempt(&app, "sam", "wrong-password").await;
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }
        clock.advance(seconds(61 - 48));
        assert_eq!(
            attempt(&app, "sam", PASSWORD).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn sign_in_limit_does_not_count_malformed_bodies() {
        let db = TestDatabase::new();
        let (app, _clock) = limited(&db, TrustedProxies::default()).await;
        for _ in 0..25 {
            let response = send(&app, "POST", "/api/session", Some("sam".into())).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        assert_eq!(
            attempt(&app, "sam", PASSWORD).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn sign_in_limit_leaves_other_routes_alone() {
        let db = TestDatabase::new();
        let (app, clock) = limited(&db, TrustedProxies::default()).await;
        fail(&app, &clock, 1, Duration::ZERO).await;
        let response = send(&app, "GET", "/api/claim", None).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
