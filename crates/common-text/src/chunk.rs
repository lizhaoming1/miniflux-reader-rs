//! Scaffold only — real implementation added in PR#2.

/// Placeholder config struct. Real chunking options (max_chars, overlap,
/// sentence boundary aware) in PR#2.
pub struct ChunkConfig {
    pub max_chars: usize,
}

/// Placeholder function. Real paragraph-level chunk with overlap in PR#2.
pub fn chunk_paragraphs(_sentences: Vec<String>, _cfg: &ChunkConfig) -> Vec<Vec<String>> {
    vec![]
}
