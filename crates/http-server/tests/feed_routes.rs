//! v0.2.0 Task 13 — `/feeds` route TDD.
//!
//! Each test owns its own temp SQLite DB + fresh AppState with Mock
//! services, then exercises the real Axum router via `tower::oneshot`.

use http_server::{build_axum_routes, AppState};
use progress_db::FeedRepository;
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

// ---------- T1: list feeds on empty DB returns [] ----------

#[tokio::test]
async fn t01_list_feeds_empty() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/feeds")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert_eq!(body, "[]", "body was: {}", body);
}

// ---------- T2: POST /feeds inserts a row ----------

#[tokio::test]
async fn t02_add_feed_inserts_row() {
    let state = test_state().await;
    let app = build_axum_routes(state.clone());
    let payload = json!({ "url": "https://added.example.com/rss" });

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/feeds")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == 201 || resp.status().is_success(),
        "got {}",
        resp.status()
    );
    let feeds = FeedRepository::new(state.db).list().await.expect("list");
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].url, "https://added.example.com/rss");
}

// ---------- T3: POST /feeds with empty url returns 400 ----------

#[tokio::test]
async fn t03_add_feed_empty_url_is_400() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let payload = json!({ "url": "" });

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/feeds")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

// ---------- T4: DELETE /feeds/999999 returns 404 ----------

#[tokio::test]
async fn t04_remove_feed_missing_is_404() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("DELETE")
                .uri("/feeds/999999")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
