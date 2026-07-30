//! Shared application state, cloned cheaply via `Arc` internals.

use std::path::PathBuf;
use std::sync::Arc;

use services::{MinifluxClient, TranslateService, TtsService};
use sqlx::SqlitePool;

/// Application state shared across all Axum handlers and the Tower
/// catch-all proxy layer.
///
/// Every field is either an `Arc` or an internally-`Arc`'d type, so
/// cloning `AppState` is cheap and safe.
#[derive(Clone)]
pub struct AppState {
    /// SQLite connection pool for `progress-db`.
    pub db: SqlitePool,
    /// Translation service (Mock or Reqwest).
    pub translate: Arc<dyn TranslateService>,
    /// TTS service (Mock or Reqwest).
    pub tts: Arc<dyn TtsService>,
    /// Miniflux HTTP client for login + request forwarding.
    pub miniflux: Arc<MinifluxClient>,
    /// Directory where uploaded EPUB files are stored.
    pub epub_dir: PathBuf,
}

impl AppState {
    /// Construct a new `AppState` from its components.
    pub fn new(
        db: SqlitePool,
        translate: Arc<dyn TranslateService>,
        tts: Arc<dyn TtsService>,
        miniflux: Arc<MinifluxClient>,
        epub_dir: PathBuf,
    ) -> Self {
        Self {
            db,
            translate,
            tts,
            miniflux,
            epub_dir,
        }
    }
}
