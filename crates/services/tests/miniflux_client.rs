//! PR#6 — `MinifluxClient` real reqwest-backed login + forward TDD.
//!
//! Uses wiremock to spin up a fake Miniflux upstream so the tests exercise
//! the real reqwest HTTP stack without hitting a live server.

use services::*;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------- login() ----------

#[tokio::test]
async fn t0_login_success_returns_session_cookie_value() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "MinifluxSession=abc123; Path=/; HttpOnly"),
        )
        .mount(&server)
        .await;

    let client = MinifluxClient::new(&server.uri());
    let session = client
        .login(&LoginForm {
            username: "admin".to_string(),
            password: "secret".to_string(),
        })
        .await
        .expect("login should succeed");

    assert_eq!(session.0, "MinifluxSession=abc123");
}

#[tokio::test]
async fn t1_login_failure_401_returns_upstream_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = MinifluxClient::new(&server.uri());
    let err = client
        .login(&LoginForm {
            username: "bad".to_string(),
            password: "creds".to_string(),
        })
        .await
        .unwrap_err();

    match err {
        ServiceError::Upstream(msg) => assert!(msg.contains("401")),
        other => panic!("expected Upstream, got {other:?}"),
    }
}

#[tokio::test]
async fn t2_login_network_error_returns_network_error() {
    // Point at a port that is guaranteed to be closed.
    let client = MinifluxClient::new("http://127.0.0.1:1");
    let err = client.login(&LoginForm::default()).await.unwrap_err();

    assert!(matches!(err, ServiceError::Network(_)));
}

// ---------- forward() ----------

#[tokio::test]
async fn t3_forward_get_returns_upstream_status_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/some/page"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html; charset=utf-8")
                .set_body_string("<html><body>Hello</body></html>"),
        )
        .mount(&server)
        .await;

    let client = MinifluxClient::new(&server.uri());
    let resp = client
        .forward(
            reqwest::Method::GET,
            "/some/page",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await
        .expect("forward GET");

    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await.expect("read body");
    assert!(body.windows(5).any(|w| w == b"Hello"));
}

#[tokio::test]
async fn t4_forward_post_sends_body_to_upstream() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/echo"))
        .and(body_string("payload-data"))
        .respond_with(ResponseTemplate::new(201).set_body_string("created"))
        .mount(&server)
        .await;

    let client = MinifluxClient::new(&server.uri());
    let resp = client
        .forward(
            reqwest::Method::POST,
            "/api/echo",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::from_static(b"payload-data"),
        )
        .await
        .expect("forward POST");

    assert_eq!(resp.status(), 201);
}

#[tokio::test]
async fn t5_forward_forwards_custom_headers_to_upstream() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/protected"))
        .and(header("x-custom", "marker-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let client = MinifluxClient::new(&server.uri());
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-custom", "marker-value".parse().unwrap());

    let resp = client
        .forward(
            reqwest::Method::GET,
            "/protected",
            headers,
            bytes::Bytes::new(),
        )
        .await
        .expect("forward with headers");

    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn t6_forward_forwards_query_string_in_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string("results"))
        .mount(&server)
        .await;

    let client = MinifluxClient::new(&server.uri());
    let resp = client
        .forward(
            reqwest::Method::GET,
            "/search?q=rust&page=2",
            reqwest::header::HeaderMap::new(),
            bytes::Bytes::new(),
        )
        .await
        .expect("forward with query");

    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await.expect("read body");
    assert_eq!(&body[..], b"results");
}
