//! # common-text
//!
//! Pure, zero-IO utilities shared by the proxy layer (miniflux injection)
//! and the EPUB reader view.
//!
//! Three submodules are provided:
//! - [`sentence`]: Chinese/English/automatic sentence splitter.
//! - [`chunk`]: Paragraph-level chunking with sentence-boundary overlap.
//! - [`bilingual`]: Source/zh pair struct and blue-accent HTML renderer.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub mod bilingual;
pub mod chunk;
pub mod sentence;

pub use bilingual::{render_bilingual_div, BilingualSentence};
pub use chunk::{chunk_paragraphs, ChunkConfig};
pub use sentence::{split_sentences, LanguageHint};
