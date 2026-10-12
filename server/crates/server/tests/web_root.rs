//! Integration tests: the router serves the web shell from a folder, and nothing else.

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, Response, StatusCode},
    Router,
};
use sidereal_server::{app, with_web_root};
use tower::ServiceExt; // for `oneshot`

mod common;
use common::{WebRoot, FOLDER_INDEX, INDEX, SCRIPT, SECRET};

async fn send(router: &Router, method: &str, uri: &str) -> Response<Body> {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::ORIGIN, "http://localhost:3000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn text(response: Response<Body>) -> String {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn serves_the_home_page_and_files() {
    let root = WebRoot::new("files");
    let router = with_web_root(app(), Some(&root.path));

    let response = send(&router, "GET", "/").await;
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers()[header::CONTENT_TYPE].to_str().unwrap();
    assert!(content_type.starts_with("text/html"));
    assert_eq!(text(response).await, INDEX);

    let response = send(&router, "HEAD", "/").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "");

    let response = send(&router, "GET", "/assets/app.js").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, SCRIPT);
}

#[tokio::test]
async fn unknown_paths_are_not_found() {
    let root = WebRoot::new("unknown");
    let router = with_web_root(app(), Some(&root.path));
    for uri in ["/no-such-page", "/api/assets"] {
        let response = send(&router, "GET", uri).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        assert_ne!(text(response).await, INDEX, "{uri}");
    }
}

#[tokio::test]
async fn folders_are_not_found() {
    let root = WebRoot::new("folders");
    let router = with_web_root(app(), Some(&root.path));
    for uri in ["/assets", "/assets/", "/assets?x=1", "//"] {
        let response = send(&router, "GET", uri).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        assert!(!response.headers().contains_key(header::LOCATION), "{uri}");
        let body = text(response).await;
        assert!(!body.contains(FOLDER_INDEX), "{uri}");
        assert!(!body.contains("app.js"), "{uri}");
    }
}

#[tokio::test]
async fn paths_cannot_leave_the_folder() {
    let root = WebRoot::new("traversal");
    let router = with_web_root(app(), Some(&root.path));
    for uri in [
        "/../secret.txt",
        "/%2e%2e/secret.txt",
        "/%2E%2E%2Fsecret.txt",
        "/assets/../../secret.txt",
        "/assets/%2e%2e/%2e%2e/secret.txt",
        "/..%5Csecret.txt",
    ] {
        let response = send(&router, "GET", uri).await;
        assert_ne!(response.status(), StatusCode::OK, "{uri}");
        assert!(!text(response).await.contains(SECRET), "{uri}");
    }
}

#[tokio::test]
async fn mutating_methods_are_not_allowed() {
    let root = WebRoot::new("methods");
    let router = with_web_root(app(), Some(&root.path));
    for uri in ["/", "/healthz", "/api/assets", "/assets/app.js"] {
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            let response = send(&router, method, uri).await;
            assert_eq!(
                response.status(),
                StatusCode::METHOD_NOT_ALLOWED,
                "{method} {uri}"
            );
        }
    }
}

#[tokio::test]
async fn a_file_cannot_replace_the_liveness_probe() {
    let root = WebRoot::new("shadow");
    let router = with_web_root(app(), Some(&root.path));
    let response = send(&router, "GET", "/healthz").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "{\"status\":\"ok\"}");
}

#[tokio::test]
async fn no_response_allows_another_origin() {
    let root = WebRoot::new("origin");
    let router = with_web_root(app(), Some(&root.path));
    for uri in ["/", "/healthz", "/assets/app.js", "/no-such-page"] {
        let response = send(&router, "GET", uri).await;
        assert!(
            !response
                .headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN),
            "{uri}"
        );
    }
}

#[tokio::test]
async fn without_a_web_root_no_file_is_served() {
    let router = with_web_root(app(), None);
    assert_eq!(
        send(&router, "GET", "/").await.status(),
        StatusCode::NOT_FOUND
    );
    let response = send(&router, "GET", "/healthz").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "{\"status\":\"ok\"}");
}
