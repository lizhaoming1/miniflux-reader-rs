//! feed-engine — RSS/Atom feed parsing, article sync, OPML support, scheduled poller.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub use parse::{ParsedArticle, ParsedFeed};
pub mod parse;
pub mod sync;
pub mod opml;
pub mod content;
