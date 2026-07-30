//! v0.2.0 Task 13 — `/settings` route TDD.

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

// ---------- T1: GET /settings returns 5 known keys ----------

#[tokio::test]
async fn t01_get_settings_returns_5_known_keys() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/settings")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 5, "expected 5 settings entries, got: {}", body);
    let keys: Vec<String> = arr
        .iter()
        .map(|e| e["key"].as_str().expect("key string").to_string())
        .collect();
    for expected in [
        "feed.poll_interval_secs",
        "feed.fetch_timeout_secs",
        "feed.user_agent",
        "translate.target_lang",
        "tts.voice",
    ] {
        assert!(
            keys.contains(&expected.to_string()),
            "missing key {} in {:?}",
            expected,
            keys
        );
    }
}

// ---------- T2: PUT /settings with unknown key returns 400 ----------

#[tokio::test]
async fn t02_put_update_settings_rejects_unknown_key() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let payload = json!({ "unknown.key": "value" });

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("PUT")
                .uri("/settings")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}
