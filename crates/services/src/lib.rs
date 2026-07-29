//! Scaffold only — real trait & impls (Mock + Reqwest + #[ignore] net test) in PR#5.

pub mod error {
    use thiserror::Error;
    #[derive(Debug, Error)] pub enum ServiceError { #[error("placeholder")] Placeholder }
    pub type Result<T, E = ServiceError> = std::result::Result<T, E>;
}
pub use error::{ServiceError, Result as SvcResult};

// ---------------- Translate -------------------------------------------------
pub mod translate {
    use super::*;
    use async_trait::async_trait;
    
    #[async_trait]
    pub trait TranslateService: Send + Sync {
        async fn translate_batch(&self, _src: Vec<String>) -> SvcResult<Vec<String>>;
    }

    /// TDD impl: returns `src` duplicated (zh = src). Real mock returns fixed zh fixtures in PR#5.
    #[derive(Clone, Default)] pub struct MockTranslateService;
    #[async_trait]
    impl TranslateService for MockTranslateService {
        async fn translate_batch(&self, src: Vec<String>) -> SvcResult<Vec<String>> { Ok(src) }
    }

    /// Network impl: `#[ignore]`-only tests. Real reqwest client + retry/timeout in PR#5.
    #[derive(Clone, Default)] pub struct ReqwestTranslateService;
    #[async_trait]
    impl TranslateService for ReqwestTranslateService {
        async fn translate_batch(&self, _src: Vec<String>) -> SvcResult<Vec<String>> { Err(ServiceError::Placeholder) }
    }
}

// ---------------- TTS -------------------------------------------------------
pub mod tts {
    use super::*;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HighlightToken {
        pub word:      String,
        pub start_ms:  u32,
        pub end_ms:    u32,
    }

    #[async_trait]
    pub trait TtsService: Send + Sync {
        async fn synthesize(&self, _text: &str) -> SvcResult<Vec<u8>>;
        async fn highlights(&self, _text: &str) -> SvcResult<Vec<HighlightToken>>;
    }

    #[derive(Clone, Default)] pub struct MockTtsService;
    #[async_trait]
    impl TtsService for MockTtsService {
        async fn synthesize(&self, text: &str) -> SvcResult<Vec<u8>> { Ok(text.as_bytes().to_vec()) }
        async fn highlights(&self, text: &str) -> SvcResult<Vec<HighlightToken>> {
            Ok(text.split_whitespace().enumerate().map(|(i, w)| HighlightToken {
                word: w.to_string(),
                start_ms: (i as u32) * 400,
                end_ms:   (i as u32 + 1) * 400,
            }).collect())
        }
    }

    #[derive(Clone, Default)] pub struct ReqwestTtsService;
    #[async_trait]
    impl TtsService for ReqwestTtsService {
        async fn synthesize(&self, _t: &str) -> SvcResult<Vec<u8>> { Err(ServiceError::Placeholder) }
        async fn highlights(&self, _t: &str) -> SvcResult<Vec<HighlightToken>> { Err(ServiceError::Placeholder) }
    }
}

// ---------------- Miniflux Client ------------------------------------------
pub mod miniflux_client {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct LoginForm { pub username: String, pub password: String }

    #[derive(Debug, Clone, Default)]
    pub struct MinifluxSession(pub String);

    /// Placeholder client. Real login()/fetch_page()/set_cookies() in PR#5.
    #[derive(Clone, Default)] pub struct MinifluxClient;
    impl MinifluxClient { pub fn placeholder() -> Self { Self } }
}
