//! # services
//!
//! Trait abstractions and two implementations (Mock for TDD, Reqwest for
//! production) for the translate and TTS services, plus a Miniflux HTTP
//! client scaffold. All public types live in dedicated submodules and are
//! re-exported from the crate root for convenience.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub mod error;
pub mod miniflux_client;
pub mod translate;
pub mod tts;

pub use error::{Result as SvcResult, ServiceError};
pub use miniflux_client::{LoginForm, MinifluxClient, MinifluxSession};
pub use translate::{MockTranslateService, ReqwestTranslateService, TranslateService};
pub use tts::{HighlightToken, MockTtsService, ReqwestTtsService, TtsService};
