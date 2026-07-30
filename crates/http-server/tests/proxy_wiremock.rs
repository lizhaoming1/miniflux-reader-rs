//! PR#6 Phase E — `MinifluxProxyLayer` real Tower catch-all TDD.
//!
//! Spins up a fake Miniflux upstream via wiremock and exercises the proxy
//! layer's CSP strip, bilingual inject, script-tag injection, header/body
//! forwarding, and 502-on-network-error behaviour.

use axum::response::IntoResponse;
use bytes::Bytes;
use http_server::proxy_layer::MinifluxProxyLayer;
use reqwest::header::HeaderMap;
use reqwest::Method;
use services::{MinifluxClient, MockTranslateService};
use std::sync::Arc;
use wiremock::matchers::{body_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a mock Miniflux upstream + a `MinifluxProxyLayer` pointing at it
/// with a 3-fixture Mock translate service.
async fn setup() -> (MockServer, MinifluxProxyLayer) {
    let server = MockServer::start().await;
    let client = MinifluxClient::new(&server.uri());
    let translate = Arc::new(MockTranslateService::with_fixed_vec(vec![
        "翻译".to_string(),
        "内容".to_string(),
        "你好".to_string(),
    ]));
    let layer = MinifluxProxyLayer::new(Arc::new(client), Some(translate));
    (server, layer)
}

/// Read the full body of an axum Response as a UTF-8 String.
async fn read_body(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .expect("body bytes");
    String::from_utf8(bytes.to_vec()).expect("utf8 body")
}

// T0: basic passthrough — non-HTML body returned unchanged
#[tokio::test]
async fn t0_get_proxied_non_html_returns_upstream_body_unchanged() {
    let (server, layer) = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/feeds.json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"feeds":[]}"#.as_bytes(), "application/json"),
        )
        .mount(&server)
        .await;

    let resp = layer
        .proxy(
            Method::GET,
            "/api/feeds.json",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("proxy");

    assert_eq!(resp.status(), 200);
    let body = read_body(resp).await;
    assert_eq!(body, r#"{"feeds":[]}"#);
}

// T1: CSP response header stripped from proxied HTML
#[tokio::test]
async fn t1_get_proxied_html_strips_content_security_policy_header() {
    let (server, layer) = setup().await;
    Mock::given(method("GET"))
        .and(path("/article/1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    "<html><body><p>hi</p></body></html>".as_bytes(),
                    "text/html; charset=utf-8",
                )
                .insert_header(
                    "content-security-policy",
                    "default-src 'none'; script-src 'self'",
                ),
        )
        .mount(&server)
        .await;

    let resp = layer
        .proxy(Method::GET, "/article/1", HeaderMap::new(), Bytes::new())
        .await
        .expect("proxy");

    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers().get("content-security-policy").is_none(),
        "CSP response header must be stripped"
    );
}

// T2: CSP meta tag stripped from proxied HTML body
#[tokio::test]
async fn t2_get_proxied_html_strips_csp_meta_tag_from_body() {
    let (server, layer) = setup().await;
    let html = "<html><head>\n\
        <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'\">\n\
        <title>x</title>\n\
        </head><body><p>hi</p></body></html>";
    Mock::given(method("GET"))
        .and(path("/article/2"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(html.as_bytes(), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;

    let resp = layer
        .proxy(Method::GET, "/article/2", HeaderMap::new(), Bytes::new())
        .await
        .expect("proxy");

    let body = read_body(resp).await;
    assert!(
        !body.to_lowercase().contains("content-security-policy"),
        "CSP meta tag must be stripped from body; body was: {body}"
    );
    assert!(body.contains("<title>x</title>"));
}

// T3: .entry-content div replaced with bilingual rendered HTML
#[tokio::test]
async fn t3_get_proxied_html_with_entry_content_injects_bilingual_zh_class() {
    let (server, layer) = setup().await;
    let html = "<html><body>\n<div class=\"entry-content\">Hello world. Good morning.</div>\n</body></html>";
    Mock::given(method("GET"))
        .and(path("/article/3"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(html.as_bytes(), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;

    let resp = layer
        .proxy(Method::GET, "/article/3", HeaderMap::new(), Bytes::new())
        .await
        .expect("proxy");

    let body = read_body(resp).await;
    assert!(
        body.contains(r#"class="sentence--zh""#),
        "expected sentence--zh class in body, got: {body}"
    );
    assert!(
        body.contains("翻译"),
        "expected zh translation fixture in body, got: {body}"
    );
}

// T4: script tag injected before </body>
#[tokio::test]
async fn t4_get_proxied_html_injects_script_before_body_close() {
    let (server, layer) = setup().await;
    let html = "<html><body><p>hi</p></body></html>";
    Mock::given(method("GET"))
        .and(path("/article/4"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(html.as_bytes(), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;

    let resp = layer
        .proxy(Method::GET, "/article/4", HeaderMap::new(), Bytes::new())
        .await
        .expect("proxy");

    let body = read_body(resp).await;
    let script_idx = body.find(r#"<script src="/_inject_rs.js"></script>"#);
    let body_close_idx = body.find("</body>");
    assert!(script_idx.is_some(), "script tag missing: {body}");
    assert!(body_close_idx.is_some(), "</body> missing: {body}");
    assert!(
        script_idx.unwrap() < body_close_idx.unwrap(),
        "script must come before </body>: {body}"
    );
}

// T5: non-HTML content type → no injection
#[tokio::test]
async fn t5_get_proxied_application_json_no_injection() {
    let (server, layer) = setup().await;
    let json = r#"{"a":1}"#;
    Mock::given(method("GET"))
        .and(path("/api/entries.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(json.as_bytes(), "application/json"))
        .mount(&server)
        .await;

    let resp = layer
        .proxy(
            Method::GET,
            "/api/entries.json",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("proxy");

    let body = read_body(resp).await;
    assert_eq!(body, json);
    assert!(!body.contains("_inject_rs.js"));
}

// T6: upstream 404 passes through
#[tokio::test]
async fn t6_get_proxied_upstream_404_passes_through_status() {
    let (server, layer) = setup().await;
    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&server)
        .await;

    let resp = layer
        .proxy(Method::GET, "/missing", HeaderMap::new(), Bytes::new())
        .await
        .expect("proxy");

    assert_eq!(resp.status(), 404);
}

// T7: POST body forwarded to upstream
#[tokio::test]
async fn t7_post_proxied_forwards_body_to_upstream() {
    let (server, layer) = setup().await;
    Mock::given(method("POST"))
        .and(path("/api/echo"))
        .and(body_string("payload-data"))
        .respond_with(ResponseTemplate::new(201).set_body_string("created"))
        .mount(&server)
        .await;

    let resp = layer
        .proxy(
            Method::POST,
            "/api/echo",
            HeaderMap::new(),
            Bytes::from_static(b"payload-data"),
        )
        .await
        .expect("proxy");

    assert_eq!(resp.status(), 201);
    let body = read_body(resp).await;
    assert_eq!(body, "created");
}

// T8: query string forwarded
#[tokio::test]
async fn t8_get_proxied_forwards_query_string_to_upstream() {
    let (server, layer) = setup().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("q", "rust"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_string("results"))
        .mount(&server)
        .await;

    let resp = layer
        .proxy(
            Method::GET,
            "/search?q=rust&page=2",
            HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("proxy");

    assert_eq!(resp.status(), 200);
    let body = read_body(resp).await;
    assert_eq!(body, "results");
}

// T9: custom header forwarded to upstream
#[tokio::test]
async fn t9_get_proxied_forwards_custom_header_to_upstream() {
    let (server, layer) = setup().await;
    Mock::given(method("GET"))
        .and(path("/protected"))
        .and(header("x-custom", "marker-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let mut headers = HeaderMap::new();
    headers.insert("x-custom", "marker-value".parse().unwrap());

    let resp = layer
        .proxy(Method::GET, "/protected", headers, Bytes::new())
        .await
        .expect("proxy");

    assert_eq!(resp.status(), 200);
}

// T10: network error (unreachable upstream) → 502 Bad Gateway
#[tokio::test]
async fn t10_network_error_unreachable_upstream_returns_502() {
    // Point at a port that is guaranteed to be closed.
    let client = MinifluxClient::new("http://127.0.0.1:1");
    let translate = Arc::new(MockTranslateService::with_fixed_vec(vec![
        "翻译".to_string()
    ]));
    let layer = MinifluxProxyLayer::new(Arc::new(client), Some(translate));

    let err = layer
        .proxy(Method::GET, "/article/x", HeaderMap::new(), Bytes::new())
        .await
        .expect_err("expected network error → 502");

    let resp = err.into_response();
    assert_eq!(resp.status(), 502);
}
