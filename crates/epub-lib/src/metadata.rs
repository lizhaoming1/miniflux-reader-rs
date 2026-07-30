//! EPUB metadata extracted from the OPF package document.

/// Metadata parsed from the OPF `<metadata>` element. `total_chapters` is
/// derived from the spine item count.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EpubMetadata {
    /// `<dc:title>` value (falls back to filename stem if absent).
    pub title: String,
    /// `<dc:creator>` value (empty string if absent).
    pub author: String,
    /// Number of spine items — i.e. the chapter count reported to callers.
    pub total_chapters: usize,
}
