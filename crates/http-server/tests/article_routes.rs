//! v0.2.0 Task 13 — `/articles` route TDD.
//!
//! Each test owns its own temp SQLite DB + fresh AppState with Mock
//! services, then exercises the real Axum router via `tower::oneshot`.

use http_server::{build_axum_routes, AppState};
use progress_db::{ArticleRepository, ArticleUpsert, FeedRepository};
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

/// Insert one feed + one article and return the article id. Used by
/// the set_read test so the route has a row to operate on.
async fn seed_one_article(state: &AppState) -> i64 {
    let feeds = FeedRepository::new(state.db.clone());
    let f = feeds
        .add("https://seed.example.com/rss")
        .await
        .expect("add");
    let arts = ArticleRepository::new(state.db.clone());
    let ups = ArticleUpsert {
        guid: "seed-guid-1".into(),
        title: "Seed Article".into(),
        url: "https://seed.example.com/1".into(),
        author: Some("tester".into()),
        summary: "summary".into(),
        content_html: Some("<p>body</p>".into()),
        published_at: "2026-07-30T00:00:00+00:00".into(),
    };
    arts.upsert(f.id, &ups).await.expect("upsert");
    arts.list_all(Some(f.id), 10, 0)
        .await
        .expect("list")
        .remove(0)
        .id
}

// ---------- T1: list articles on empty DB returns [] ----------

#[tokio::test]
async fn t01_list_articles_empty() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/articles")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert_eq!(body, "[]", "body was: {}", body);
}

// ---------- T2: GET /articles/999999 returns 404 ----------

#[tokio::test]
async fn t02_get_article_missing_is_404() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/articles/999999")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---------- T3: POST /articles/:id/read flips the read flag ----------

#[tokio::test]
async fn t03_set_article_read_flips_flag() {
    let state = test_state().await;
    let art_id = seed_one_article(&state).await;
    let app = build_axum_routes(state.clone());
    let payload = json!({ "read": true });

    let resp = app
        .oneshot(
            http::Request::builder()
                .method("POST")
                .uri(format!("/articles/{}/read", art_id))
                .header("content-type", "application/json")
                .body(axum::body::Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert!(body.contains("\"ok\":true"), "body was: {}", body);

    let got = ArticleRepository::new(state.db)
        .get(art_id)
        .await
        .expect("get");
    assert!(got.is_read, "article should be marked read in DB");
}

// ---------- T4: GET /articles/unread-count returns {"count":0} on empty DB ----------

#[tokio::test]
async fn t04_unread_count_empty_is_zero() {
    let state = test_state().await;
    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri("/articles/unread-count")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert_eq!(body, r#"{"count":0}"#, "body was: {}", body);
}

// ---------- T5: GET /articles/:id includes translated zh content ----------

#[tokio::test]
async fn t05_get_article_includes_translated_zh() {
    let state = test_state().await;
    let feeds = FeedRepository::new(state.db.clone());
    let f = feeds
        .add("https://seed.example.com/rss")
        .await
        .expect("add");
    let arts = ArticleRepository::new(state.db.clone());
    let ups = ArticleUpsert {
        guid: "seed-guid-trans".into(),
        title: "Translate Test".into(),
        url: "https://seed.example.com/trans".into(),
        author: Some("tester".into()),
        summary: "summary".into(),
        // Plain text with no terminator so en_split produces exactly 1 sentence.
        content_html: Some("Hello".into()),
        published_at: "2026-07-30T00:00:00+00:00".into(),
    };
    arts.upsert(f.id, &ups).await.expect("upsert");
    let art_id = arts
        .list_all(Some(f.id), 10, 0)
        .await
        .expect("list")
        .remove(0)
        .id;

    let app = build_axum_routes(state);
    let resp = app
        .oneshot(
            http::Request::builder()
                .method("GET")
                .uri(format!("/api/v1/articles/{}", art_id))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = body_string(resp).await;
    assert!(
        body.contains("sentence--zh"),
        "missing zh paragraph class: {}",
        body
    );
    assert!(
        body.contains("你好"),
        "missing translated text '你好': {}",
        body
    );
}
