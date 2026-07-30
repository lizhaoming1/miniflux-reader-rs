//! PR#2 Task 2.2 — RED phase: 3 failing tests for paragraph chunking.
//!
//! Scaffold `chunk_paragraphs` returns `vec![]`, so every assertion here
//! fails until GREEN phase provides an implementation.

use common_text::{chunk_paragraphs, ChunkConfig};

#[test]
fn max_chars_smaller_than_first_sentence_keeps_sentence_intact() {
    // 1 sentence of 4 chars; max_chars=1 (strictly smaller) → the chunk must
    // still contain exactly that one sentence (we never split inside a
    // sentence; if one sentence overflows it is still kept alone).
    let sents = vec!["abcd".to_string()];
    let cfg = ChunkConfig {
        max_chars: 1,
        overlap_sentences: 0,
    };
    let chunks = chunk_paragraphs(sents, &cfg);
    assert_eq!(chunks.len(), 1, "overflowing 1-sentence input → 1 chunk");
    assert_eq!(chunks[0], vec!["abcd".to_string()]);
}

#[test]
fn overlap_one_repeats_last_sentence_across_boundary() {
    // Two sentences of exactly 10 chars each, max_chars=10, overlap=1
    // → after chunk 1 holds s0, chunk 2 starts with overlap of s0 then s1.
    let s0 = "0123456789".to_string();
    let s1 = "abcdefghij".to_string();
    let sents = vec![s0.clone(), s1.clone()];
    let cfg = ChunkConfig {
        max_chars: 10,
        overlap_sentences: 1,
    };
    let chunks = chunk_paragraphs(sents, &cfg);
    assert!(
        chunks.len() >= 2,
        "need at least 2 chunks; got {:?}",
        chunks
    );
    assert_eq!(chunks[0], vec![s0.clone()]);
    assert_eq!(
        chunks[1],
        vec![s0, s1],
        "chunk 2 must start with overlap of last 1 sentence from chunk 1"
    );
}

#[test]
fn empty_sentences_returns_zero_chunks() {
    let cfg = ChunkConfig {
        max_chars: 100,
        overlap_sentences: 0,
    };
    let chunks = chunk_paragraphs(vec![], &cfg);
    assert_eq!(chunks.len(), 0, "empty input → empty chunk vec, not [[]]");
}
