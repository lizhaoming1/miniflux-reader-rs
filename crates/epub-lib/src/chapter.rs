//! Chapter struct extracted from an EPUB spine item.

/// One chapter: zero-based spine index, a human-readable title (from the
/// TOC nav or `<dc:title>` of the spine item), and the raw HTML text of
/// the chapter's content document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    /// Zero-based index into the OPF spine.
    pub idx: usize,
    /// Human-readable chapter title.
    pub title: String,
    /// Raw HTML text of the chapter content document.
    pub html_text: String,
}
