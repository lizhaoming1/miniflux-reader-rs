//! v0.2.0 Task 13 — `/opml` route TDD.

use http_server::{build_axum_routes, AppState};
use serde_json::json;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// ---------- helpers ----------

async fn test_state() -> AppState {
    let dir = tempfile::tempdir().expect("tempdir");
    let dir_path = dir.keep();
    let db_path = dir_path.join("test.db");
    let db_url = format!("sqlite://{}", db_path.display());
    let pool = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::from_str(&db_url)
                .expect("opts")
                .create_if_missing(true),
        )
        .await
        .expect("connect");
    progress_db::run_migrations(&pool).await.expect("migrate");
    std::env::set_var("RUST_EPUB_BOOKS_DIR", dir_path.join("epubs"));
    AppState::new(
        pool,
        Arc::new(services::MockTranslateService::with_fixed_vec(vec![
            "你好".to_string()
        ])),
        Arc::new(services::MockTtsService::default()),
        dir_path.join("epubs"),
    )
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .expect("read body");
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

// ---------- T1: GET /opml/export on empty DB returns valid OPML XML ----------

#[tokio::test]
async fn t01_export_empty_returns_valid_xml() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/opml/export")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("text/xml"),
        "content-type should contain text/xml, got: {}",
        ct
    );
    let body = body_string(resp).await;
    assert!(
        body.contains("<opml"),
        "body should contain <opml> tag: {}",
        body
    );
}

// ---------- T2: import OPML with 1 feed, then export contains the URL ----------

#[tokio::test]
async fn t02_import_then_export_roundtrip() {
    let state = test_state().await;
    let app = build_axum_routes(state.clone());
    let target_url = "https://roundtrip.example.com/feed.xml";
    let opml = format!(
        r#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Subs</title></head>
  <body>
    <outline type="rss" text="RT" xmlUrl="{}"/>
  </body>
</opml>"#,
        target_url
    );

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/opml/import")
                .header("content-type", "text/xml")
                .body(axum::body::Body::from(opml))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["added"], json!(1), "import should add 1 feed: {}", body);

    // Now export and verify the URL appears.
    let app2 = build_axum_routes(state);
    let resp2 = app2
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/opml/export")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp2.status(), 200);
    let body2 = body_string(resp2).await;
    assert!(
        body2.contains(target_url),
        "export should contain the imported URL: {}",
        body2
    );
}
