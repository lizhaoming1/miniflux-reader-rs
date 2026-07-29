//! Scaffold only — real axum Router, Tower MinifluxProxyLayer (catch-all,
//! CSP strip, scraper inject), AppState, AppConfig loader, tracing init,
//! http-error IntoResponse, ALL added in PR#6 (TDD: RED tests first commit).

// ---------- Module declarations (previously empty, now with real placeholders) -----
pub mod config {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct MinifluxCfg {
        pub url:      String,
        pub username: String,
        pub password: String,
    }

    /// Real AppConfig struct in PR#6 matches `rust-config.example.json` schema.
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct AppConfig {
        pub listen_addr: String,
        pub miniflux:    MinifluxCfg,
    }

    use thiserror::Error;
    #[derive(Debug, Error)] pub enum LoadCfgError { #[error("placeholder")] Placeholder }

    /// Placeholder loader. Real impl: serde_json::from_reader(File::open) + validate paths in PR#6.
    pub fn load(_path: &str) -> Result<AppConfig, LoadCfgError> { Ok(AppConfig::default()) }
}
pub use config::{AppConfig, MinifluxCfg, LoadCfgError};

pub mod state {
    /// Placeholder state. Real `Arc<Inner>` with sqlx Pool + TranslateService + TtsService in PR#6.
    #[derive(Clone, Default)] pub struct AppState;
    impl AppState { pub fn placeholder() -> Self { Self } }
}
pub use state::AppState;

pub mod routes {
    use axum::Router;
    use super::AppState;

    /// Placeholder router. Real `build_axum_routes` + LeptosRouter + catch-all in PR#6.
    pub fn build_axum_routes(_state: AppState) -> Router { Router::new() }
}
pub use routes::build_axum_routes;

pub mod proxy_layer {
    /// Placeholder tower service. Real MinifluxProxyLayer (strip CSP, scraper inject bilingual) in PR#6.
    #[derive(Clone, Default)] pub struct MinifluxProxyLayer;
}
pub use proxy_layer::MinifluxProxyLayer;

pub mod error {
    use axum::response::{IntoResponse, Json};
    use http::StatusCode;
    use serde_json::json;

    /// Placeholder error. Real per-crate thiserror → IntoResponse mapping in PR#6.
    #[derive(Debug)] pub enum AppHttpError { Placeholder }
    impl IntoResponse for AppHttpError {
        fn into_response(self) -> axum::response::Response {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!( {"code": "PLACEHOLDER", "msg": "PR#6"} ))).into_response()
        }
    }
}

pub mod logging {
    /// Placeholder init_tracing. Real `tracing_subscriber::fmt().with_env_filter(...)` in PR#6.
    pub fn init_tracing() {}
}
pub use logging::init_tracing;
