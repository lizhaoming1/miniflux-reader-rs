//! # services
//!
//! Trait abstractions and two implementations (Mock for TDD, Reqwest for
//! production) for the translate and TTS services. MinifluxClient was
//! removed in v0.2.0 in favour of a built-in RSS engine. All public
//! types live in dedicated submodules and are re-exported from the crate root.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all)]

pub mod error;
pub mod translate;
pub mod tts;

pub use error::{Result as SvcResult, ServiceError};
pub use translate::{MockTranslateService, ReqwestTranslateService, TranslateService};
pub use tts::{HighlightToken, MockTtsService, ReqwestTtsService, TtsService};
