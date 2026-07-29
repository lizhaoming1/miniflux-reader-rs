//! # common-text
//!
//! Pure, zero-IO utilities shared by the proxy layer (miniflux injection)
//! and the EPUB reader view.
//!
//! While the crate is still scaffolding (PR#1) the submodules (sentence,
//! chunk, bilingual) are empty, so missing_docs is set to `warn` rather
//! than `deny`. PR#2 flips this back to `deny`.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

pub mod sentence;
pub mod chunk;
pub mod bilingual;

pub use sentence::{split_sentences, LanguageHint};
pub use chunk::{chunk_paragraphs, ChunkConfig};
pub use bilingual::{render_bilingual_div, BilingualSentence};
