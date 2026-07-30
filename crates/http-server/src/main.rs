//! Binary entry point for `http-server`.
//!
//! Loads `rust-config.json`, initialises tracing, creates the SQLite pool
//! and services, builds the Axum router with feed/article/opml/settings
//! routes, and starts serving. The background RSS poller task is spawned
//! before entering the serve loop.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use feed_engine::sync::{start_poller, SyncConfig};
use http_server::{init_tracing_with_filter, load, AppState};
use leptos::prelude::*;
use services::{ReqwestTranslateService, ReqwestTtsService};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

/// Fallback handler: any path not matched by an explicit API route is
/// treated as a page request and rendered through the Leptos SSR pipeline.
/// The rendered `<App/>` body is wrapped in an HTML shell that loads the
/// WASM hydration bundle from `/pkg/app.js` (when built via cargo-leptos).
async fn leptos_ssr_fallback(req: Request<Body>) -> Response {
    let _path = req.uri().path().to_string();
    // Render the Leptos `App` to an HTML string via the SSR runtime.
    // `RenderHtml::to_html()` is exported from `leptos::prelude::*`.
    let app_html = view! { <leptos_app::App /> }.to_html().to_string();
    // Wrap with a minimal HTML5 shell + pkg script reference.
    // CSS / styling is intentionally inline / minimal for P0.
    let full = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>miniflux-reader-rs v0.2.0</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
           max-width: 900px; margin: 0 auto; padding: 1rem 1.25rem; color: #222; }}
    .app-nav {{ margin-bottom: 1rem; font-size: 0.95rem; }}
    .app-nav a {{ color: #2563eb; text-decoration: none; margin-right: 0.25rem; }}
    .app-nav a:hover {{ text-decoration: underline; }}
    .app-header {{ border-bottom: 1px solid #e5e7eb; padding-bottom: 0.75rem; margin-bottom: 1.25rem; }}
    .app-header h1 {{ margin: 0; font-size: 1.5rem; }}
    .tagline {{ margin: 0.25rem 0 0; color: #6b7280; font-size: 0.9rem; }}
    .route-fallback, .empty-state {{ color: #6b7280; padding: 2rem; text-align: center; }}
    .article-list {{ list-style: none; padding: 0; }}
    .article-row {{ padding: 0.6rem 0.25rem; border-bottom: 1px solid #f3f4f6; }}
    .article-row.is-read .article-title {{ color: #6b7280; font-weight: normal; }}
    .article-title {{ display: block; font-weight: 600; color: #111827; text-decoration: none; }}
    .article-title:hover {{ text-decoration: underline; }}
    .article-meta {{ display: block; margin-top: 0.2rem; font-size: 0.8rem; color: #6b7280; }}
    .feed-list {{ list-style: none; padding: 0; }}
    .feed-card {{ padding: 0.75rem 0.5rem; border-bottom: 1px solid #f3f4f6; display: flex; align-items: center; gap: 0.75rem; }}
    .feed-title {{ flex: 1; color: #111827; font-weight: 600; text-decoration: none; }}
    .feed-unread {{ background: #dbeafe; color: #1e40af; padding: 0.1rem 0.5rem; border-radius: 9999px; font-size: 0.8rem; font-weight: 600; }}
    .feed-delete {{ background: #fee2e2; color: #991b1b; border: 1px solid #fecaca; padding: 0.2rem 0.6rem; border-radius: 0.3rem; cursor: pointer; }}
    .add-feed-form, .upload-form, .settings-form {{ display: flex; flex-direction: column; gap: 0.6rem; margin: 1rem 0; padding: 1rem; background: #f9fafb; border-radius: 0.5rem; }}
    .add-feed-form label, .settings-form label {{ display: flex; flex-direction: column; gap: 0.25rem; font-size: 0.9rem; }}
    .add-feed-form input, .settings-form input, .upload-form input {{ padding: 0.4rem 0.5rem; border: 1px solid #d1d5db; border-radius: 0.3rem; font-size: 0.95rem; }}
    .add-feed-form button, .settings-form button, .upload-form button {{ padding: 0.5rem 1rem; background: #2563eb; color: #fff; border: none; border-radius: 0.3rem; cursor: pointer; font-weight: 600; }}
    .add-feed-form button:hover, .settings-form button:hover, .upload-form button:hover {{ background: #1d4ed8; }}
    .settings-form fieldset {{ border: 1px solid #e5e7eb; border-radius: 0.4rem; padding: 0.75rem 1rem; margin: 0; }}
    .settings-form legend {{ padding: 0 0.3rem; font-weight: 600; color: #374151; }}
    .opml-actions {{ display: flex; gap: 1rem; align-items: center; margin: 0.75rem 0; }}
    .opml-export {{ color: #2563eb; text-decoration: none; font-weight: 500; }}
    .bookshelf {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem; }}
    .book-card {{ padding: 1rem; border: 1px solid #e5e7eb; border-radius: 0.5rem; background: #fafafa; }}
    .book-card__title {{ margin: 0 0 0.25rem; font-size: 1rem; }}
    .book-card__author {{ margin: 0 0 0.75rem; font-size: 0.85rem; color: #6b7280; }}
    .book-card a {{ color: #2563eb; text-decoration: none; font-weight: 500; }}
    .bookshelf--empty {{ color: #6b7280; padding: 2rem; text-align: center; }}
    .book-reader__header {{ display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #e5e7eb; padding-bottom: 0.5rem; margin-bottom: 1rem; }}
    .book-reader__header a {{ color: #2563eb; text-decoration: none; }}
    .book-reader__title {{ font-weight: 600; }}
    .book-reader__content {{ line-height: 1.7; padding: 1rem; background: #fffbeb; border-radius: 0.3rem; border: 1px solid #fde68a; }}
    .article-reader {{ line-height: 1.75; }}
    .sentence {{ margin: 0.35rem 0; }}
    .sentence--zh {{ margin: 0.25rem 0 0.75rem; border-left: 3px solid #3f87ff; padding-left: 0.6rem; color: #1e40af; }}
  </style>
</head>
<body>
  {app_html}
  <script type="module" src="/pkg/app.js"></script>
</body>
</html>"#
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(full),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ---- Configuration ----
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "rust-config.json".to_string());
    let cfg = load(&config_path).map_err(|e| {
        eprintln!("Failed to load config from {config_path}: {e}");
        eprintln!("Usage: http-server [config.json]");
        e
    })?;

    // ---- Tracing ----
    init_tracing_with_filter(&cfg.log_filter);
    tracing::info!(
        "config loaded from {config_path}, listening on {}",
        cfg.listen_addr
    );

    // ---- Database ----
    if let Some(parent) = std::path::Path::new(&cfg.paths.db).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&cfg.paths.db)
            .create_if_missing(true),
    )
    .await?;
    progress_db::run_migrations(&pool).await?;
    tracing::info!("database ready at {}", cfg.paths.db);

    // ---- EPUB directory ----
    std::fs::create_dir_all(&cfg.paths.epub_dir).ok();
    std::env::set_var("RUST_EPUB_BOOKS_DIR", &cfg.paths.epub_dir);

    // ---- Services ----
    let translate: Arc<dyn services::TranslateService> = Arc::new(ReqwestTranslateService);
    let tts: Arc<dyn services::TtsService> = Arc::new(ReqwestTtsService);

    // ---- Application state ----
    let state = AppState::new(
        pool.clone(),
        translate,
        tts,
        std::path::PathBuf::from(&cfg.paths.epub_dir),
    );

    // ---- Background poller ----
    {
        let pool_c = pool.clone();
        let sync_cfg = SyncConfig {
            fetch_timeout: Duration::from_secs(cfg.feed.fetch_timeout_secs),
            user_agent: Some(cfg.feed.user_agent.clone()),
        };
        let interval = Duration::from_secs(cfg.feed.poll_interval_secs);
        tokio::spawn(async move { start_poller(pool_c, interval, sync_cfg).await });
        tracing::info!(
            interval_s = cfg.feed.poll_interval_secs,
            "RSS poller started"
        );
    }

    // ---- Server ----
    // Build the unified router: explicit /api/v1/* + /healthz, the
    // /pkg/<file> static wasm mount, and the Leptos SSR fallback for
    // any unmatched page route.
    let pkg_dir = cfg.paths.pkg_dir.clone().unwrap_or_else(|| "pkg".to_string());
    let pkg_path = std::path::PathBuf::from(&pkg_dir);
    if !pkg_path.exists() {
        tracing::warn!(
            pkg_dir = %pkg_path.display(),
            "/pkg/ static dir not found — wasm hydration will 404"
        );
    }
    let app = http_server::build_page_router(
        state,
        &pkg_path,
        axum::routing::any(leptos_ssr_fallback),
    );
    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!("server started on {}", cfg.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
