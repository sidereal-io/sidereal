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

