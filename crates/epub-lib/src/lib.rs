//! Scaffold only — real implementations in PR#3–5.

// ---------- epub-lib public types -------------------------------------------
pub use self::error::{EpubError, Result as EpubResult};
pub mod parse {
    use super::*;

    /// Placeholder reader. Real `EpubReader::open(path)` → list chapters in PR#3.
    pub struct EpubReader;
    impl EpubReader {
        pub fn placeholder() -> Self {
            EpubReader
        }
    }
}
pub mod upload {
    use super::*;
    pub fn safe_name_for(_file_name: &str) -> String {
        "placeholder-safename".to_string()
    }
    pub fn save_upload_to_disk(_bytes: &[u8], _file_name: &str) -> EpubResult<String> {
        Ok(String::new())
    }
}
pub mod metadata {
    /// Placeholder struct. Real `title/author/total_chapters` in PR#3.
    pub struct EpubMetadata {
        pub title: String,
    }
}
pub mod chapter {
    /// Placeholder chapter. Real `idx, title, html_text` in PR#3.
    pub struct Chapter {
        pub idx: usize,
    }
}
pub mod error {
    use thiserror::Error;
    #[derive(Debug, Error)]
    pub enum EpubError {
        #[error("placeholder")]
        Placeholder,
    }
    pub type Result<T, E = EpubError> = std::result::Result<T, E>;
}
