//! # common-text
//!
//! Pure, zero-IO utilities shared by the proxy layer (miniflux injection)
//! and the EPUB reader view. No dependency on SQLx, reqwest, axum, or leptos
//! — which means this crate compiles in seconds and can be unit-tested in
//! complete isolation.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub mod sentence;
pub mod chunk;
pub mod bilingual;

pub use sentence::{split_sentences, LanguageHint};
pub use chunk::{chunk_paragraphs, ChunkConfig};
pub use bilingual::{render_bilingual_div, BilingualSentence};
