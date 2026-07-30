//! PR#8 — Workspace-level integration tests (≥3).
//!
//! Each test exercises the full Axum router (`build_axum_routes`) with real
//! cross-crate wiring: `http-server` → `epub-lib` / `progress-db` /
//! `services` / `common-text`. Scenarios:
//!
//! * **T0 (L3+L4)** — EPUB upload + progress save, then AppState restart
//!   (new `SqlitePool` against the same on-disk file) + progress read-back
//!   matching `chapter_idx=5, percent=50`. Simulates a user closing the
//!   browser and reopening the book.
//!
//! **Test isolation note:** T0 calls `epub_lib::save_upload_to_disk`, which
//! reads the process-global `RUST_EPUB_BOOKS_DIR` env var. To avoid racing
//! with other upload tests, run this file serially:
//! `cargo test -p http-server --test integration_full -- --test-threads=1`.

use http_server::{build_axum_routes, AppState};
use serde_json::json;
use services::{MockTranslateService, MockTtsService};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// ---------- shared helpers ----------

/// Build a `multipart/form-data` body with a single `file` field.
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

/// Read the full body of an axum `Response` as a UTF-8 `String`.
async fn body_string(resp: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .expect("read body");
    String::from_utf8(bytes.to_vec()).expect("utf8")
}

/// Open a SQLite pool at `db_path` (creating the file if missing) and run
/// all workspace migrations. Idempotent — safe to call again on an already
/// migrated file (e.g. when "restarting" AppState).
async fn fresh_pool(db_path: &Path) -> SqlitePool {
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
    pool
}

// ---------- T0 (L3+L4): progress persists across AppState restart ----------

/// Verify that reading progress survives an AppState restart: upload an
/// EPUB, save progress `{chapter_idx=5, percent=50}`, drop the AppState,
/// build a brand new AppState pointing at the same on-disk SQLite file +
/// EPUB dir, then GET the progress and assert both fields match.
#[tokio::test]
async fn t0_progress_persists_across_appstate_restart() {
    // A single TempDir holds both the SQLite DB and the EPUB dir so both
    // survive the AppState drop (simulating a browser close / reopen).
    let dir = tempfile::tempdir().expect("tempdir");
    let dir_path = dir.keep();
    let db_path = dir_path.join("progress.db");
    let epub_dir = dir_path.join("epubs");
    std::fs::create_dir_all(&epub_dir).ok();
    // `save_upload_to_disk` reads this env var; set it for this test.
    std::env::set_var("RUST_EPUB_BOOKS_DIR", &epub_dir);

    // ---- AppState #1: upload EPUB + save progress ----
    {
        let pool = fresh_pool(&db_path).await;
        let state = AppState::new(
            pool,
            Arc::new(MockTranslateService::with_fixed_vec(vec![
                "你好".to_string()
            ])),
            Arc::new(MockTtsService::default()),
            epub_dir.clone(),
        );
        let app = build_axum_routes(state);

        // Upload a fake EPUB (bytes don't matter — only the safe_name does).
        let (ct, body) = multipart_body("file", "persist-test.epub", b"fake-epub-bytes");
        let resp = app
            .clone()
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
        assert_eq!(resp.status(), 200, "upload should succeed");

        // Save progress {chapter_idx=5, percent=50}.
        let payload = json!({
            "epub_path": "persist-test.epub",
            "chapter_idx": 5,
            "scroll_pos": 500,
            "percent": 50.0,
            "overall": 10.0
        });
        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("POST")
                    .uri("/epub/api/progress/persist-test.epub")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "save progress should succeed");
    } // AppState #1 dropped — SqlitePool closed, file remains on disk.

    // ---- AppState #2: brand new pool, same DB file + same epub dir ----
    {
        let pool = fresh_pool(&db_path).await; // reopen same file
        let state = AppState::new(
            pool,
            Arc::new(MockTranslateService::with_fixed_vec(vec![
                "你好".to_string()
            ])),
            Arc::new(MockTtsService::default()),
            epub_dir.clone(),
        );
        let app = build_axum_routes(state);

        let resp = app
            .oneshot(
                http::Request::builder()
                    .method("GET")
                    .uri("/epub/api/progress/persist-test.epub")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "progress should exist after restart");
        let body = body_string(resp).await;
        let v: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(v["chapter_idx"], 5, "chapter_idx must persist: {body}");
        assert!(
            (v["percent"].as_f64().unwrap() - 50.0).abs() < 0.01,
            "percent must persist: {body}"
        );
    }
}
