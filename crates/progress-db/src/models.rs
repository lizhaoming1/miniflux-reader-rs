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

/// An RSS / Atom feed subscription.
#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct Feed {
    /// Primary key.
    pub id: i64,
    /// Absolute URL of the RSS / Atom feed document.
    pub url: String,
    /// Human readable title (parsed from `<title>`).
    pub title: String,
    /// URL of the source web site.
    pub site_url: String,
    /// ISO-8601 timestamp of the last successful fetch, or `None`.
    pub last_fetched: Option<String>,
    /// Last error message, or `None` if last fetch succeeded.
    pub fetch_error: Option<String>,
}

/// A single article fetched from a feed.
#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct Article {
    /// Primary key.
    pub id: i64,
    /// Owning feed id.
    pub feed_id: i64,
    /// Globally unique id within the feed (from RSS `<guid>` or Atom `<id>`).
    pub guid: String,
    /// Article title.
    pub title: String,
    /// Original article URL.
    pub url: String,
    /// Article author if known.
    pub author: Option<String>,
    /// Short summary (from `<description>`).
    pub summary: String,
    /// Full HTML content if provided by feed (from `<content:encoded>`).
    pub content_html: Option<String>,
    /// ISO-8601 publication timestamp.
    pub published_at: String,
    /// True if the user has marked this article read.
    #[sqlx(rename = "read")]
    pub is_read: bool,
}
