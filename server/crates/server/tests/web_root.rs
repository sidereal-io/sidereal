//! Integration tests: the router serves the web shell from a folder, and nothing else.

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, Response, StatusCode},
    Router,
};
use sidereal_server::{app, with_web_root};
use tower::ServiceExt; // for `oneshot`

mod common;
use common::{WebRoot, ASSET_PAGE, FOLDER_INDEX, INDEX, SCRIPT, SECRET};

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

#[tokio::test]
async fn an_asset_page_address_answers_with_index_html() {
    let root = WebRoot::new("asset-page");
    let router = with_web_root(app(), Some(&root.path));
    let response = send(&router, "GET", ASSET_PAGE).await;
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers()[header::CONTENT_TYPE].to_str().unwrap();
    assert!(content_type.starts_with("text/html"), "{content_type}");
    assert_eq!(text(response).await, INDEX);

    let response = send(&router, "HEAD", ASSET_PAGE).await;
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers()[header::CONTENT_TYPE].to_str().unwrap();
    assert!(content_type.starts_with("text/html"), "{content_type}");
    assert_eq!(text(response).await, "");
}

#[tokio::test]
async fn a_file_at_an_asset_page_address_is_not_served() {
    let root = WebRoot::new("reserved");
    std::fs::write(root.path.join(&ASSET_PAGE[1..]), "the added file").unwrap();
    let router = with_web_root(app(), Some(&root.path));
    let response = send(&router, "GET", ASSET_PAGE).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, INDEX);

    let encoded = ASSET_PAGE.replacen("/assets/0", "/assets/%30", 1);
    assert_eq!(encoded, "/assets/%30b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091");
    for method in ["GET", "HEAD"] {
        let response = send(&router, method, &encoded).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method}");
        let body = text(response).await;
        assert_ne!(body, INDEX, "{method}");
        assert!(!body.contains("the added file"), "{method}");
    }
}

/// Each `src` and `href` path the shell's `index.html` names.
fn named_files() -> Vec<String> {
    let mut paths = Vec::new();
    for attribute in ["src=\"", "href=\""] {
        for (start, _) in INDEX.match_indices(attribute) {
            let rest = &INDEX[start + attribute.len()..];
            paths.push(rest[..rest.find('"').unwrap()].to_owned());
        }
    }
    paths
}

#[tokio::test]
async fn the_web_shells_own_files_still_load() {
    let root = WebRoot::new("own-files");
    let router = with_web_root(app(), Some(&root.path));
    let named = named_files();
    assert_eq!(named.len(), 2, "{named:?}");
    for path in named {
        let response = send(&router, "GET", &path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let on_disk = std::fs::read_to_string(root.path.join(&path[1..])).unwrap();
        assert_eq!(text(response).await, on_disk, "{path}");
        let response = send(&router, "HEAD", &path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn other_paths_under_assets_are_not_asset_pages() {
    let root = WebRoot::new("not-pages");
    let router = with_web_root(app(), Some(&root.path));
    let upper = ASSET_PAGE.replace(
        "0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091",
        "0B6F3C2E-8D1A-4F7B-9C3E-2A5D6E7F8091",
    );
    for uri in [
        "/assets/not-an-id".to_owned(),
        upper,
        format!("{ASSET_PAGE}/"),
        format!("{ASSET_PAGE}/original"),
        "/assets/0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f809".to_owned(),
        "/assets/0b6f3c2e8d1a-4f7b-9c3e-2a5d6e7f80911".to_owned(),
        format!("/x{ASSET_PAGE}"),
    ] {
        let response = send(&router, "GET", &uri).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        assert_ne!(text(response).await, INDEX, "{uri}");
    }
}
