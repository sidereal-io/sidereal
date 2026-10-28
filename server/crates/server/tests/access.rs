//! The claim and session routes, driven through the router against a fresh
//! database per test.
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use sidereal_core::database::{initialize, Database};
use sidereal_server::app_with_database;
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
    app_with_database(database(db).await)
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
    let response = send(&app_with_database(database), "GET", "/api/claim", None).await;
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
    let response = claim(&app_with_database(database), "sam", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await, r#"{"error":"unavailable"}"#);
    assert_eq!(users(&db).await, 0);
}
