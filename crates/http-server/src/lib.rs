//! # http_server
//!
//! Axum HTTP server for the unified reading platform. Wires feeds,
//! articles, OPML, settings, EPUB, translate, and TTS routes together
//! into a single Axum router with a shared [`AppState`].

pub mod config;
pub mod error;
pub mod logging;
pub mod routes;
pub mod state;

pub use config::{load, AppConfig, FeedCfg, PathsCfg, TranslateCfg, TtsCfg};
pub use error::AppHttpError;
pub use logging::{init_tracing, init_tracing_with_filter};
pub use routes::{build_axum_routes, build_page_router};
pub use state::AppState;
