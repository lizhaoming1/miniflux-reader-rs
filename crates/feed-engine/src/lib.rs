//! feed-engine — RSS/Atom feed parsing, article sync, OPML support, scheduled poller.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

use thiserror::Error;

pub use parse::{ParsedArticle, ParsedFeed};
pub use sync::{start_poller, sync_all_feeds, sync_feed, SyncConfig};
pub mod content;
pub mod opml;
pub mod parse;
pub mod sync;

/// Errors returned by `feed-engine` operations.
#[derive(Debug, Error)]
pub enum FeedEngineError {
    /// HTTP / network error fetching feed or article content.
    #[error("network error: {0}")]
    Network(String),
    /// Feed XML was parseable but structurally invalid.
    #[error("feed parse error: {0}")]
    Parse(String),
    /// Upstream status not success.
    #[error("upstream returned status {0}")]
    UpstreamStatus(u16),
    /// Database error forwarded from progress-db.
    #[error("{0}")]
    Db(#[from] progress_db::DbError),
    /// Request timeout.
    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),
}

/// Convenience Result alias for this crate.
pub type Result<T> = std::result::Result<T, FeedEngineError>;
