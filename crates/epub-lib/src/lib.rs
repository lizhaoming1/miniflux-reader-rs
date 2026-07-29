//! # epub-lib
//!
//! Reads EPUB 2.x/3.x packages from disk or an uploaded byte stream, builds
//! the manifest + spine + NCX table of contents, and exposes chapter text
//! for downstream rendering.
//! The upload helper writes files to `rust-epub-books/<safe_name>.epub`
//! and sanitises the filename against directory traversal.

#![forbid(unsafe_code)]

pub mod parse;
pub mod upload;
pub mod metadata;
pub mod chapter;
pub mod error;

pub use error::{EpubError, Result};
pub use metadata::EpubMetadata;
pub use chapter::Chapter;
pub use parse::EpubReader;
pub use upload::{safe_name_for, save_upload_to_disk};
