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
//! * **T1 (L2+L5)** — wiremock fake Miniflux serves an article with 3
//!   Chinese segments inside `.entry-content`; the full Axum fallback →
//!   `MinifluxProxyLayer` → `MockTranslateService` → bilingual-inject
//!   pipeline runs end-to-end and the response contains exactly 3
//!   `class="sentence--zh"` paragraphs.
//! * **T2 (L1+L6)** — `POST /mf-login` → 200 `Set-Cookie: MF_SESSION_RS=…`
//!   (HttpOnly), then `GET /tts?text=Hi` → 200 non-empty `audio/mpeg` body
//!   via `MockTtsService`.
//!
//! **Test isolation note:** T0 calls `epub_lib::save_upload_to_disk`, which
//! reads the process-global `RUST_EPUB_BOOKS_DIR` env var. To avoid racing
//! with other upload tests, run this file serially:
//! `cargo test -p http-server --test integration_full -- --test-threads=1`.

use http_server::{build_axum_routes, AppState};
use serde_json::json;
use services::{MinifluxClient, MockTranslateService, MockTtsService};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
            Arc::new(MinifluxClient::new("http://localhost:1")),
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
            Arc::new(MinifluxClient::new("http://localhost:1")),
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

// ---------- T1 (L2+L5): full proxy pipeline with bilingual inject ----------

/// Verify the end-to-end proxy pipeline through the real Axum router
/// fallback: wiremock serves an HTML article with 3 Chinese segments in
/// `.entry-content`; after proxy + translate + bilingual inject the
/// response contains exactly 3 `class="sentence--zh"` paragraphs and the
/// injected `/_inject_rs.js` script tag.
#[tokio::test]
async fn t1_proxy_pipeline_injects_bilingual_for_three_segments() {
    let server = MockServer::start().await;

    // Fake Miniflux article: 3 Chinese-terminated sentences inside
    // .entry-content — the proxy layer will split, translate, and render
    // one bilingual block per sentence.
    let html = "<html><body>\n\
        <div class=\"entry-content\">你好世界。早上好。今天天气不错。</div>\n\
        </body></html>";
    Mock::given(method("GET"))
        .and(path("/article/zh"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(html.as_bytes(), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;

    // Fresh SQLite pool (location irrelevant — T1 never touches EPUB
    // upload or progress save; it only exercises the proxy fallback).
    let db_dir = tempfile::tempdir().expect("db tempdir");
    let pool = fresh_pool(&db_dir.keep().join("t1.db")).await;
    let epub_dir = tempfile::tempdir().expect("epub tempdir");

    let state = AppState::new(
        pool,
        Arc::new(MockTranslateService::with_fixed_vec(vec![
            "翻译1".to_string(),
            "翻译2".to_string(),
            "翻译3".to_string(),
        ])),
        Arc::new(MockTtsService::default()),
        Arc::new(MinifluxClient::new(&server.uri())),
        epub_dir.keep(),
    );
    let app = build_axum_routes(state);

    // GET /article/zh misses all explicit routes → Axum fallback →
    // MinifluxProxyLayer → wiremock upstream → post-process → response.
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/article/zh")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;

    // Exactly 3 sentence--zh paragraphs (one per Chinese segment).
    let zh_count = body.matches(r#"class="sentence--zh""#).count();
    assert_eq!(
        zh_count, 3,
        "expected exactly 3 bilingual zh paragraphs, got {zh_count}; body: {body}"
    );
    // Script injection also ran (proves the full pipeline executed).
    assert!(
        body.contains(r#"<script src="/_inject_rs.js"></script>"#),
        "inject script should be present: {body}"
    );
}

// ---------- T2 (L1+L6): login set-cookie + TTS stream ----------

/// Verify the L1 (login) + L6 (TTS) flow in one router: `POST /mf-login`
/// with JSON credentials → 200 + `Set-Cookie: MF_SESSION_RS=…; HttpOnly`,
/// then `GET /tts?text=Hi` → 200 non-empty `audio/mpeg` body.
#[tokio::test]
async fn t2_login_sets_cookie_then_tts_streams_audio() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "MinifluxSession=abc567; Path=/; HttpOnly"),
        )
        .mount(&server)
        .await;

    let db_dir = tempfile::tempdir().expect("db tempdir");
    let pool = fresh_pool(&db_dir.keep().join("t2.db")).await;
    let epub_dir = tempfile::tempdir().expect("epub tempdir");

    let state = AppState::new(
        pool,
        Arc::new(MockTranslateService::with_fixed_vec(vec![
            "你好".to_string()
        ])),
        Arc::new(MockTtsService::default()),
        Arc::new(MinifluxClient::new(&server.uri())),
        epub_dir.keep(),
    );
    let app = build_axum_routes(state);

    // ---- Step 1: POST /mf-login → Set-Cookie MF_SESSION_RS ----
    let login_payload = json!({
        "username": "admin",
        "password": "secret"
    });
    let resp = app
        .clone()
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/mf-login")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(login_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "login should succeed");
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        set_cookie.starts_with("MF_SESSION_RS="),
        "Set-Cookie should start with MF_SESSION_RS=, got: {set_cookie}"
    );
    assert!(
        set_cookie.contains("HttpOnly"),
        "cookie should be HttpOnly: {set_cookie}"
    );

    // ---- Step 2: GET /tts?text=Hi → non-empty audio bytes ----
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/tts?text=Hi")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "tts should succeed");
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("audio"),
        "content-type should contain audio, got: {ct}"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    assert!(
        !bytes.is_empty(),
        "tts body should be non-empty (mock returns MP3 header + text)"
    );
}
