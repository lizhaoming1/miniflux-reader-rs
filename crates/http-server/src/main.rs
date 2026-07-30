//! Binary entry point for `http-server`.
//!
//! Loads `rust-config.json`, initialises tracing, creates the SQLite pool
//! and services, builds the Axum router with feed/article/opml/settings
//! routes, and starts serving. The background RSS poller task is spawned
//! before entering the serve loop.

use std::sync::Arc;
use std::time::Duration;

use feed_engine::sync::{start_poller, SyncConfig};
use http_server::{build_axum_routes, init_tracing_with_filter, load, AppState};
use services::{ReqwestTranslateService, ReqwestTtsService};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

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
    let app = build_axum_routes(state);
    let listener = tokio::net::TcpListener::bind(&cfg.listen_addr).await?;
    tracing::info!("server started on {}", cfg.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
