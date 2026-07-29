//! # progress-db
//!
//! SQLx + SQLite persistence for two tables (see `/migrations/001*` and
//! `/migrations/002*`):
//!   - `reading_progress` — upsert per unique `epub_path`, keyed on the
//!     sanitised EPUB storage location returned by `epub-lib::safe_name_for`.
//!   - `books`           — metadata (title, author, chapter count, size).
//! Schema is **NOT** backward compatible with the Python version:
//! per Scheme C the two stacks live in separate directories and share
//! nothing.

#![forbid(unsafe_code)]

pub mod models;
pub mod repository;
pub mod migrate;
pub mod error;

pub use error::{DbError, Result};
pub use models::{ReadingProgress, Book};
pub use repository::{ProgressRepository, BookRepository};
pub use migrate::run_migrations;
