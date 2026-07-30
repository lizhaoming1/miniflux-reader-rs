//! Leptos 0.7 views: Bookshelf, BookReader, ArticleReader + server fns.
//!
//! Compiles both as SSR (axum) and as CSR (wasm hydration). SSR rendering is
//! done via `RenderHtml::to_html()` from `leptos::prelude::*`.

pub mod app;
pub mod article_list;
pub mod article_reader;
pub mod book_reader;
pub mod bookshelf;
pub mod error_boundary;
pub mod feed_list;
pub mod server_fns;
pub mod settings_page;

pub use app::App;
pub use article_list::ArticleList;
pub use article_reader::{ArticleReader, BilingualToggle};
pub use book_reader::{BookReader, TtsPlayer};
pub use bookshelf::{Bookshelf, UploadForm};
pub use feed_list::{AddFeedForm, FeedList, OpmlActions};
pub use settings_page::SettingsPage;

/// Lightweight book metadata used by `<Bookshelf/>` for rendering cards.
///
/// This is a UI-only view model; the canonical `Book` record lives in
/// `progress-db`. The frontend receives this shape from the server and does
/// not need the full DB row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BookInfo {
    /// Sanitized on-disk filename (e.g. `my-book.epub`).
    pub safe_name: String,
    /// Human-readable title parsed from the EPUB metadata.
    pub title: String,
    /// Author parsed from the EPUB metadata, or `""` if absent.
    pub author: String,
}

/// Lightweight feed metadata used by `<FeedList/>` for rendering.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedInfo {
    /// Feed primary key.
    pub id: i64,
    /// Human-readable feed title.
    pub title: String,
    /// URL of the source web site.
    pub site_url: String,
    /// Number of unread articles in this feed.
    pub unread_count: i64,
}

/// Lightweight article metadata used by `<ArticleList/>` for rendering rows.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArticleSummary {
    /// Article primary key.
    pub id: i64,
    /// Owning feed id.
    pub feed_id: i64,
    /// Title of the owning feed (denormalised for display).
    pub feed_title: String,
    /// Article title.
    pub title: String,
    /// Original article URL.
    pub url: String,
    /// ISO-8601 publication timestamp.
    pub published_at: String,
    /// Whether the article has been marked read.
    pub read: bool,
}

/// UI toggle for the bilingual rendering mode of `<ArticleReader/>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ToggleLang {
    /// Show only the original source text.
    Original,
    /// Show both source and Chinese translation.
    Bilingual,
}

/// Calculate how many times a debounced save would fire for a sequence of
/// timestamps.
///
/// A "fire" occurs at the end of each group of calls where consecutive calls
/// are within `window_ms` of each other. Calls that are `>= window_ms` apart
/// start a new group (and thus a new fire).
///
/// This is a pure function for testing the debounce logic without needing
/// timers or async runtimes.
pub fn debounce_fire_count(calls: &[u64], window_ms: u64) -> usize {
    if calls.is_empty() {
        return 0;
    }
    let mut count = 1;
    for i in 1..calls.len() {
        if calls[i].saturating_sub(calls[i - 1]) >= window_ms {
            count += 1;
        }
    }
    count
}
