//! # epub-lib
//!
//! EPUB parsing (zip + OPF XML), chapter extraction, and safe-name upload
//! utility. Depends only on [`common_text`] for sentence-level processing
//! of extracted chapter text; all IO is explicit (no hidden filesystem
//! state).

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub mod chapter;
pub mod error;
pub mod metadata;
pub mod parse;
pub mod upload;

pub use chapter::Chapter;
pub use error::{EpubError, Result as EpubResult};
pub use metadata::EpubMetadata;
pub use parse::EpubReader;
pub use upload::{safe_name_for, save_upload_to_disk};
