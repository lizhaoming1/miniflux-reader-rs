//! Scaffold only — real thiserror impl added in PR#3.

use thiserror::Error;

/// All EPUB-layer errors. Enum variants are added in PR#3; for now the
/// scaffold keeps the single placeholder variant so downstream crates
/// can already depend on `epub_lib::Result<T>` in their signatures.
#[derive(Debug, Error)]
pub enum EpubError {
    /// Placeholder — replaced with real variants in PR#3.
    #[error("placeholder")]
    Placeholder,
}

pub type Result<T, E = EpubError> = std::result::Result<T, E>;
