//! Error types for the services layer.
//!
//! All translate / TTS / miniflux client implementations convert their
//! underlying failures into [`ServiceError`] so callers only need to handle
//! one error enum.

use std::time::Duration;
use thiserror::Error;

/// All service-layer errors.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// The translate batch contained more segments than the configured
    /// fixture buffer can satisfy (payload too large).
    #[error("translate batch too long: {0} segments exceed fixture capacity")]
    TextTooLong(usize),

    /// The operation did not complete within the configured timeout.
    #[error("operation timed out after {0:?}")]
    Timeout(Duration),

    /// A low-level network failure (connection reset, DNS, TLS, …).
    #[error("network error: {0}")]
    Network(String),

    /// The upstream service returned an error response (4xx / 5xx or
    /// malformed payload).
    #[error("upstream service error: {0}")]
    Upstream(String),
}

/// Convenience `Result` alias used by every service trait.
pub type Result<T, E = ServiceError> = std::result::Result<T, E>;
