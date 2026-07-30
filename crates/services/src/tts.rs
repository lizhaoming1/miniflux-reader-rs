//! TTS service trait + Mock (deterministic) and Reqwest (stub) impls.

use crate::error::{Result as SvcResult, ServiceError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// A single word-level highlight window inside a synthesized audio clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightToken {
    /// The surface form of the highlighted word.
    pub word: String,
    /// Start offset of the word within the audio, in milliseconds.
    pub start_ms: u32,
    /// End offset of the word within the audio, in milliseconds (`> start_ms`).
    pub end_ms: u32,
}

/// Abstraction over a text-to-speech backend.
///
/// Implementations synthesize raw audio bytes for a piece of text and also
/// expose word-level highlight timing metadata.
#[async_trait]
pub trait TtsService: Send + Sync {
    /// Synthesize `text` into raw audio bytes (e.g. MP3).
    async fn synthesize(&self, text: &str) -> SvcResult<Vec<u8>>;
    /// Return per-word highlight tokens for `text`.
    async fn highlights(&self, text: &str) -> SvcResult<Vec<HighlightToken>>;
}

/// Input length (in bytes) above which the mock treats the text as
/// "very long" and honours the configured timeout by failing fast.
const LONG_TEXT_THRESHOLD: usize = 1024;

/// Inner shared state for [`MockTtsService`].
#[derive(Default)]
struct MockTtsInner {
    /// Optional simulated timeout; when set, very long inputs fail fast.
    timeout: Option<Duration>,
}

/// Deterministic mock TTS service used by tests and offline development.
///
/// `synthesize` returns a minimal valid MP3 frame header (`0xFF 0xFB 0x90
/// 0x00`) followed by the raw input bytes, so callers can assert on both the
/// header marker and the payload. `highlights` emits one token per
/// whitespace-separated word, spaced 400 ms apart with monotonically
/// increasing start times.
///
/// `Clone` is cheap (inner state lives behind an `Arc`), and clones share
/// the same configuration.
#[derive(Clone, Default)]
pub struct MockTtsService {
    inner: Arc<MockTtsInner>,
}

impl MockTtsService {
    /// Create a mock that simulates a timeout for very long inputs.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            inner: Arc::new(MockTtsInner {
                timeout: Some(timeout),
            }),
        }
    }
}

#[async_trait]
impl TtsService for MockTtsService {
    async fn synthesize(&self, text: &str) -> SvcResult<Vec<u8>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(t) = self.inner.timeout {
            if text.len() > LONG_TEXT_THRESHOLD {
                return Err(ServiceError::Timeout(t));
            }
        }
        let mut out: Vec<u8> = vec![0xFF, 0xFB, 0x90, 0x00];
        out.extend_from_slice(text.as_bytes());
        Ok(out)
    }

    async fn highlights(&self, text: &str) -> SvcResult<Vec<HighlightToken>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let mut tokens = Vec::new();
        for (i, w) in text.split_whitespace().enumerate() {
            let start_ms = (i as u32) * 400;
            let end_ms = start_ms + 400;
            tokens.push(HighlightToken {
                word: w.to_string(),
                start_ms,
                end_ms,
            });
        }
        Ok(tokens)
    }
}

/// Reqwest-backed production TTS client.
///
/// Not exercised by unit tests; the real HTTP wiring lands in a later PR.
#[derive(Clone, Default)]
pub struct ReqwestTtsService;

#[async_trait]
impl TtsService for ReqwestTtsService {
    async fn synthesize(&self, _text: &str) -> SvcResult<Vec<u8>> {
        Err(ServiceError::Network(
            "reqwest tts backend not yet wired".into(),
        ))
    }

    async fn highlights(&self, _text: &str) -> SvcResult<Vec<HighlightToken>> {
        Err(ServiceError::Network(
            "reqwest tts backend not yet wired".into(),
        ))
    }
}
