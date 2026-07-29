//! # services
//!
//! Houses the two external-service integrations that need deterministic
//! TDD coverage but call real third-party networks in production:
//!   - `TranslateService` — batched parallel sentence translation
//!     with retry + per-call timeout (no multi-backend fallback per YAGNI)
//!   - `TtsService`       — audio MP3 generation + word-level timestamps
//!     for the highlight JSON endpoint
//! Each trait ships a `Mock*` impl that returns fixed fixtures (used by
//! **all** default `cargo test` runs) and a `Reqwest*` impl gated under
//! `#[ignore]` — the CI never needs outbound network.

#![forbid(unsafe_code)]

pub mod translate;
pub mod tts;
pub mod miniflux_client;
pub mod error;

pub use error::{ServiceError, Result};
pub use translate::{TranslateService, MockTranslateService, ReqwestTranslateService};
pub use tts::{TtsService, MockTtsService, ReqwestTtsService, HighlightToken};
pub use miniflux_client::{MinifluxClient, MinifluxSession, LoginForm};
