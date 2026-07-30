//! PR#6 Phase D — `routes.rs` EPUB + healthz route TDD.
//!
//! Each test owns its own temp SQLite DB + fresh AppState with Mock
//! services, then exercises the real Axum router via `tower::oneshot`.

use http_server::{build_axum_routes, AppState};
use progress_db::ReadingProgress;
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
        Arc::new(services::MinifluxClient::new("http://localhost:1")),
        dir_path.join("epubs"),
    )
}

/// Build a `multipart/form-data` body with a single field.
fn multipart_body(field_name: &str, filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----FormBoundary7MA4YWxkTrZu0gW";
    let content_type = format!("multipart/form-data; boundary={}", boundary);
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n\r\n",
            field_name, filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    (content_type, body)
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .expect("read body");
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

// ---------- T0: healthz ----------

#[tokio::test]
async fn t0_healthz_returns_200_with_ok_true() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/healthz")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert!(body.contains("\"ok\":true"), "body was: {}", body);
}

// ---------- T1: epub upload ----------

#[tokio::test]
async fn t1_upload_returns_200_with_safe_name() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let (ct, body) = multipart_body("file", "book.epub", b"fake-epub-bytes");

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/upload")
                .header("content-type", ct)
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert!(body.contains("safe_name"), "body was: {}", body);
}

// ---------- T2: save progress ----------

#[tokio::test]
async fn t2_save_progress_returns_200_ok() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let payload = json!({
        "epub_path": "book.epub",
        "chapter_idx": 2,
        "scroll_pos": 100,
        "percent": 50.0,
        "overall": 25.0
    });

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/api/progress/book.epub")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert!(body.contains("\"ok\":true"), "body was: {}", body);
}

// ---------- T3: get progress after save ----------

#[tokio::test]
async fn t3_get_progress_after_save_returns_saved() {
    let state = test_state().await;
    let app = build_axum_routes(state);

    let payload = json!({
        "epub_path": "book.epub",
        "chapter_idx": 4,
        "scroll_pos": 200,
        "percent": 60.0,
        "overall": 30.0
    });
    let _ = app
        .clone()
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/api/progress/book.epub")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/epub/api/progress/book.epub")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["epub_path"], "book.epub");
}

// ---------- T4: get nonexistent progress -> 404 ----------

#[tokio::test]
async fn t4_get_nonexistent_progress_returns_404() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/epub/api/progress/does-not-exist.epub")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---------- T5: save progress with malformed JSON -> 400 ----------

#[tokio::test]
async fn t5_save_progress_malformed_json_returns_400() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/api/progress/book.epub")
                .header("content-type", "application/json")
                .body(axum::body::Body::from("{ this is not json }"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

// ---------- T6: get progress returns correct chapter_idx and percent ----------

#[tokio::test]
async fn t6_get_progress_returns_correct_chapter_idx_and_percent() {
    let state = test_state().await;
    let app = build_axum_routes(state);

    let p = ReadingProgress {
        epub_path: "metrics.epub".into(),
        chapter_idx: 7,
        scroll_pos: 333,
        percent: 77.5,
        overall: 44.0,
    };
    let _ = app
        .clone()
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/api/progress/metrics.epub")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(serde_json::to_string(&p).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/epub/api/progress/metrics.epub")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    let v: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(v["chapter_idx"], 7);
    assert!((v["percent"].as_f64().unwrap() - 77.5).abs() < 0.01);
}

// ---------- T7: upload with no file field -> 400 ----------

#[tokio::test]
async fn t7_upload_no_file_field_returns_400() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let (ct, body) = multipart_body("not_file", "x.txt", b"hello");

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/upload")
                .header("content-type", ct)
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}
