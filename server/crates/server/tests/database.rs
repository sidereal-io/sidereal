use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sidereal_core::database::initialize;
use sidereal_server::{app_with_database, with_web_root};
use sqlx::{Connection, Executor, PgConnection};
use std::{process::Command, time::Duration};
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

mod common;
use common::WebRoot;

#[tokio::test]
async fn health_routes_are_fixed_and_methods_do_not_mutate() {
    let db = TestDatabase::new();
    let database = initialize(
        db.options.clone(),
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    let app = app_with_database(database.clone());
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let before: String = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(m) ORDER BY version)::text FROM public._sqlx_migrations m",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    for (route, body) in [
        ("/healthz", "{\"status\":\"ok\"}"),
        ("/readyz", "{\"status\":\"ready\"}"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), body);
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(route)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
    }
    let after: String = sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(m) ORDER BY version)::text FROM public._sqlx_migrations m",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(before, after);
    conn.execute("UPDATE public._sqlx_migrations SET version=99 WHERE version=2")
        .await
        .unwrap();
    for (route, status, body) in [
        ("/healthz", StatusCode::OK, "{\"status\":\"ok\"}"),
        (
            "/readyz",
            StatusCode::SERVICE_UNAVAILABLE,
            "{\"status\":\"not_ready\"}",
        ),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), body);
    }
    database.close().await;
}

#[test]
fn startup_errors_do_not_disclose_credentials_or_bind_http() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    for value in [
        None,
        Some("invalid-distinctive-secret"),
        Some(
            "postgres://distinctive-user:distinctive-password@127.0.0.1:9/database?sslmode=disable",
        ),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sidereal-server"));
        command
            .env_remove("DATABASE_URL")
            .env("PORT", port.to_string())
            .env("DATABASE_STARTUP_TIMEOUT_SECONDS", "1");
        if let Some(value) = value {
            command.env("DATABASE_URL", value);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        let captured = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        for secret in [
            "distinctive-secret",
            "distinctive-user",
            "distinctive-password",
            "postgres://",
        ] {
            assert!(!captured.contains(secret), "credential disclosure");
        }
        assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_err());
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_sidereal-server"));
    command.env(
        "DATABASE_URL",
        "postgres://distinctive-user:distinctive-password@localhost/database?sslmode=prefer",
    );
    let output = command.output().unwrap();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("distinctive"));
}

#[test]
fn authentication_errors_are_safe() {
    use sqlx::ConnectOptions;
    let db = TestDatabase::new();
    let options = db
        .options
        .clone()
        .password("distinctive-authentication-secret");
    let url = options.to_url_lossy().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
        .env("DATABASE_URL", &url)
        .env("DATABASE_STARTUP_TIMEOUT_SECONDS", "2")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let captured = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!captured.contains("distinctive-authentication-secret"));
    assert!(!captured.contains(&url));
    assert!(!captured.contains("password authentication failed"));
}

#[tokio::test]
async fn web_root_leaves_the_probes_fixed_and_refuses_mutation() {
    let db = TestDatabase::new();
    let database = initialize(
        db.options.clone(),
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    // The web root holds files named `healthz` and `readyz`.
    let root = WebRoot::new("probes");
    let app = with_web_root(app_with_database(database.clone()), Some(&root.path));
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let rows = "SELECT (SELECT jsonb_agg(to_jsonb(m) ORDER BY version) FROM public._sqlx_migrations m)::text
        || (SELECT jsonb_agg(to_jsonb(s)) FROM public.sidereal_metadata s)::text";
    let before: String = sqlx::query_scalar(rows).fetch_one(&mut conn).await.unwrap();
    for (route, body) in [
        ("/healthz", "{\"status\":\"ok\"}"),
        ("/readyz", "{\"status\":\"ready\"}"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(to_bytes(response.into_body(), 1024).await.unwrap(), body);
    }
    for route in ["/", "/healthz", "/readyz", "/api/assets"] {
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(route)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
    }
    let after: String = sqlx::query_scalar(rows).fetch_one(&mut conn).await.unwrap();
    assert_eq!(before, after);
    database.close().await;
}

#[tokio::test]
async fn bad_web_root_fails_before_the_database_changes() {
    use sqlx::ConnectOptions;
    let db = TestDatabase::new();
    let url = db.options.to_url_lossy().to_string();
    let root = WebRoot::new("startup");
    std::fs::remove_file(root.path.join("index.html")).unwrap();
    for web_root in [root.path.join("no-such-folder"), root.path.clone()] {
        let output = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
            .env("DATABASE_URL", &url)
            .env("WEB_ROOT", &web_root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("WEB_ROOT"));
    }
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let tables: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema')",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(tables, 0);
    conn.close().await.unwrap();
}
