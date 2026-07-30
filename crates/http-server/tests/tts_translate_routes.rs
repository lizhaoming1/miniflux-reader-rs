//! PR#6 Phase D — `routes.rs` TTS + translate route TDD.

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

async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .expect("read body");
    bytes.to_vec()
}

// ---------- T0: tts?text=hello ----------

#[tokio::test]
async fn t0_tts_hello_returns_200_audio() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/tts?text=hello")
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
        ct.contains("audio"),
        "content-type should contain audio, got: {}",
        ct
    );
    let body = body_bytes(resp).await;
    assert!(!body.is_empty(), "body should not be empty");
}

// ---------- T1: tts?text= (empty) ----------

#[tokio::test]
async fn t1_tts_empty_text_returns_200_empty_body() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/tts?text=")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_bytes(resp).await;
    assert!(body.is_empty(), "empty text should produce empty body");
}

// ---------- T2: translate?text=hello ----------

#[tokio::test]
async fn t2_translate_hello_returns_translation_string() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/translate?text=hello")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert_eq!(text, "你好");
}

// ---------- T3: tts_highlight with 3 words ----------

#[tokio::test]
async fn t3_tts_highlight_three_words_returns_array_len_3() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let payload = json!({ "text": "word1 word2 word3" });

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/tts_highlight")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("json array");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 3);
}

// ---------- T4: tts_highlight with malformed JSON -> 400 ----------

#[tokio::test]
async fn t4_tts_highlight_malformed_json_returns_400() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/tts_highlight")
                .header("content-type", "application/json")
                .body(axum::body::Body::from("{ not json"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}
