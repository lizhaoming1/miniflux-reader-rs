//! Data models: `ReadingProgress` and `Book` with serde + sqlx::FromRow.

use serde::{Deserialize, Serialize};

/// Reading progress for a single EPUB file. Keyed on `epub_path` (the
/// unique safe-name path under `rust-epub-books/`).
#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct ReadingProgress {
    /// Unique key: the on-disk path of the EPUB file.
    pub epub_path: String,
    /// Current chapter index (zero-based).
    pub chapter_idx: i32,
    /// Scroll position within the chapter (pixels or percentage, caller-defined).
    pub scroll_pos: i32,
    /// Percentage within the current chapter (0.0–100.0).
    pub percent: f32,
    /// Overall percentage across the whole book (0.0–100.0).
    pub overall: f32,
}

/// Metadata for an uploaded EPUB book. Keyed on `safe_name`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct Book {
    /// Safe-name identifier (from `epub_lib::safe_name_for`).
    pub safe_name: String,
    /// Book title (from OPF `<dc:title>`).
    pub title: String,
    /// Book author (from OPF `<dc:creator>`), may be empty.
    pub author: String,
    /// Total number of spine chapters.
    pub total_chapters: i64,
    /// File size in bytes.
    pub file_size: i64,
}
