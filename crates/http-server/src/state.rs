//! Shared application state, cloned cheaply via `Arc` internals or
//! cheaply-cloneable structs.

use std::path::PathBuf;
use std::sync::Arc;

use progress_db::{ArticleRepository, BookRepository, FeedRepository, SettingsRepository};
use services::{TranslateService, TtsService};
use sqlx::SqlitePool;

/// Application state shared across all Axum handlers.
///
/// Every field is either an `Arc` or an internally-Arc'd type, so
/// cloning `AppState` is cheap and safe.
#[derive(Clone)]
pub struct AppState {
    /// SQLite connection pool for `progress-db`.
    pub db: SqlitePool,
    /// Translation service (Mock or Reqwest).
    pub translate: Arc<dyn TranslateService>,
    /// TTS service (Mock or Reqwest).
    pub tts: Arc<dyn TtsService>,
    /// Feed repository (feeds table).
    pub feed_repo: FeedRepository,
    /// Article repository (articles table).
    pub article_repo: ArticleRepository,
    /// Book repository (books table).
    pub book_repo: BookRepository,
    /// Runtime settings repository.
    pub settings_repo: SettingsRepository,
    /// Directory where uploaded EPUB files are stored.
    pub epub_dir: PathBuf,
}

impl AppState {
    /// Construct a new `AppState` from its components.
    pub fn new(
        db: SqlitePool,
        translate: Arc<dyn TranslateService>,
        tts: Arc<dyn TtsService>,
        epub_dir: PathBuf,
    ) -> Self {
        let feed_repo = FeedRepository::new(db.clone());
        let article_repo = ArticleRepository::new(db.clone());
        let book_repo = BookRepository::new(db.clone());
        let settings_repo = SettingsRepository::new(db.clone());
        Self {
            db,
            translate,
            tts,
            feed_repo,
            article_repo,
            book_repo,
            settings_repo,
            epub_dir,
        }
    }
}
