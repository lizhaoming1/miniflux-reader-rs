//! PR#6 Phase D — `routes.rs` status code TDD.
//!
//! With the proxy fallback (Phase F), unmatched routes are forwarded to
//! the Miniflux upstream. These tests verify the fallback behaviour:
//! upstream 404 passes through, and malformed JSON still returns 400.

use http_server::{build_axum_routes, AppState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------- helpers ----------

async fn test_state(miniflux_url: &str) -> AppState {
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
        Arc::new(services::MinifluxClient::new(miniflux_url)),
        dir_path.join("epubs"),
    )
}

// ---------- T0: unknown route → upstream 404 passes through ----------

#[tokio::test]
async fn t0_unknown_route_upstream_404_passes_through() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/nonexistent-route"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let state = test_state(&server.uri()).await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/nonexistent-route")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---------- T1: malformed JSON → 400 ----------

#[tokio::test]
async fn t1_malformed_json_returns_400() {
    let state = test_state("http://localhost:1").await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri("/epub/api/progress/book.epub")
                .header("content-type", "application/json")
                .body(axum::body::Body::from("{ malformed json }"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}
