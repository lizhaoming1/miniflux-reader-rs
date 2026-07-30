//! Translation service trait + Mock (fixture-backed) and Reqwest (stub) impls.

use crate::error::{Result as SvcResult, ServiceError};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Abstraction over a batch translation backend.
///
/// Implementations translate a batch of source-language strings into the
/// target language. The Mock impl is fixture-backed for deterministic TDD;
/// the Reqwest impl calls a real upstream in production.
#[async_trait]
pub trait TranslateService: Send + Sync {
    /// Translate `src` (one entry per source segment) into the target
    /// language, returning translations in the same order.
    async fn translate_batch(&self, src: Vec<String>) -> SvcResult<Vec<String>>;
}

/// Fixture-backed mock used by tests and offline development.
///
/// Holds a fixed pool of target-language (zh) translations. Each call to
/// [`TranslateService::translate_batch`] returns the leading slice of
/// fixtures matching the input indices; if the batch is larger than the
/// fixture pool the call fails with [`ServiceError::TextTooLong`].
///
/// `call_count` is shared across `Clone`s via `Arc`, so any clone observes
/// the total number of calls across all clones sharing the counter.
#[derive(Clone, Default)]
pub struct MockTranslateService {
    /// Fixed target-language fixtures indexed by source segment position.
    fixtures: Vec<String>,
    /// Shared, atomic call counter incremented on every `translate_batch`.
    call_count: Arc<AtomicUsize>,
}

impl MockTranslateService {
    /// Create a new mock backed by the supplied fixture vector.
    pub fn with_fixed_vec(v: Vec<String>) -> Self {
        Self {
            fixtures: v,
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Number of times `translate_batch` has been invoked on this mock
    /// (or any clone sharing its counter).
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl TranslateService for MockTranslateService {
    async fn translate_batch(&self, src: Vec<String>) -> SvcResult<Vec<String>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        if src.is_empty() {
            return Ok(Vec::new());
        }
        if src.len() > self.fixtures.len() {
            return Err(ServiceError::TextTooLong(src.len()));
        }
        Ok(self.fixtures[..src.len()].to_vec())
    }
}

/// Reqwest-backed production translator.
///
/// Not exercised by unit tests; the real HTTP wiring lands in a later PR.
#[derive(Clone, Default)]
pub struct ReqwestTranslateService;

#[async_trait]
impl TranslateService for ReqwestTranslateService {
    async fn translate_batch(&self, _src: Vec<String>) -> SvcResult<Vec<String>> {
        Err(ServiceError::Network(
            "reqwest translate backend not yet wired".into(),
        ))
    }
}
