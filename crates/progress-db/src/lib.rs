//! # progress-db
//!
//! sqlx SQLite layer for reading-progress + books tables. Fully isolated
//! from the Python project's schema (Plan C: `rust-*` prefixed paths and
//! databases). Provides async repository structs over `sqlx::SqlitePool`.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub mod error;
pub mod migrate;
pub mod models;
pub mod repository;

pub use error::{DbError, Result as DbResult};
pub use migrate::run_migrations;
pub use models::{Article, Book, Feed, ReadingProgress};
pub use repository::{
    ArticleRepository, ArticleUpsert, BookRepository, FeedRepository, ProgressRepository, SettingsRepository,
};
