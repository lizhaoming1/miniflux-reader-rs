//! Error types for the EPUB layer.

use thiserror::Error;

/// All EPUB-layer errors. Variants cover IO, zip, OPF/XML parse, and
/// directory-traversal defense failures.
#[derive(Debug, Error)]
pub enum EpubError {
    /// Underlying IO failure (file not found, permission denied, …).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// `zip` crate returned an error while reading the EPUB archive.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// OPF / container XML could not be parsed.
    #[error("opf xml parse error: {0}")]
    Xml(String),

    /// The submitted filename attempted directory traversal (`..`, absolute
    /// path, NUL byte, …) and was rejected by `safe_name_for`.
    #[error("unsafe filename rejected: {0}")]
    UnsafeFilename(String),

    /// A required EPUB entry (mimetype, container.xml, OPF, chapter href)
    /// was missing from the archive.
    #[error("missing entry in epub: {0}")]
    MissingEntry(String),
}

/// Convenience `Result` alias for the EPUB layer.
pub type Result<T, E = EpubError> = std::result::Result<T, E>;
