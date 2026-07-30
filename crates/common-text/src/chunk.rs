//! Paragraph-level chunking with sentence-aware overlap.
//!
//! Input sentences are accumulated into chunks up to `ChunkConfig.max_chars`
//! per chunk. If closing a chunk leaves a tail, the next chunk prepends
//! `overlap_sentences` sentences from it so context is preserved. A single
//! sentence that overflows `max_chars` is still kept alone — we never split
//! inside a sentence boundary.

/// Configuration for paragraph-level chunking.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum characters allowed inside a single chunk (sum of sentence
    /// lengths; padding is never added). If a single sentence exceeds this
    /// limit it is kept alone rather than being split mid-sentence.
    pub max_chars: usize,
    /// Number of sentences carried from the end of a closed chunk to the
    /// head of the next chunk so context is preserved across boundaries.
    pub overlap_sentences: usize,
}

/// Partition `sentences` into chunks per `cfg`. Empty input → empty output.
///
/// # Algorithm
/// 1. Accumulate sentences while `Σ len(sentence)` fits in `max_chars`.
/// 2. When adding the next sentence would exceed `max_chars` (and there is
///    already ≥1 sentence in the running chunk), close the chunk, start a
///    new one whose prefix is the tail `overlap_sentences` of the closed
///    chunk, then continue appending.
pub fn chunk_paragraphs(sentences: Vec<String>, cfg: &ChunkConfig) -> Vec<Vec<String>> {
    if sentences.is_empty() {
        return Vec::new();
    }
    let mut chunks: Vec<Vec<String>> = Vec::new();
    let mut running: Vec<String> = Vec::new();
    let mut running_chars: usize = 0;
    for s in sentences {
        let s_len = s.chars().count();
        let would_overflow = running_chars.saturating_add(s_len) > cfg.max_chars;
        if would_overflow && !running.is_empty() {
            // Close the current chunk, prepare the overlap prefix.
            let closed = std::mem::take(&mut running);
            running_chars = 0;
            let overlap_n = cfg.overlap_sentences.min(closed.len());
            if overlap_n > 0 {
                let tail: Vec<String> = closed[closed.len() - overlap_n..].to_vec();
                running_chars = tail.iter().map(|t| t.chars().count()).sum();
                running.extend(tail);
            }
            chunks.push(closed);
        }
        running_chars = running_chars.saturating_add(s_len);
        running.push(s);
    }
    if !running.is_empty() {
        chunks.push(running);
    }
    chunks
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn single_sentence_always_forms_one_chunk_even_on_overflow() {
        let r = chunk_paragraphs(
            vec!["xx".to_string()],
            &ChunkConfig { max_chars: 1, overlap_sentences: 0 },
        );
        assert_eq!(r, vec![vec!["xx".to_string()]]);
    }
}
