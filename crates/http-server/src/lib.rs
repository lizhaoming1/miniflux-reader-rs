//! # http-server
//!
//! Axum HTTP server with Leptos SSR + Hydration and a Tower catch-all
//! Miniflux proxy layer. This crate wires together all other workspace
//! crates (`common-text`, `epub-lib`, `progress-db`, `services`) into a
//! single binary.

pub mod config;
pub mod error;
pub mod logging;
pub mod proxy_layer;
pub mod routes;
pub mod state;

pub use config::{load, AppConfig, LoadCfgError, MinifluxCfg, PathsCfg, TranslateCfg, TtsCfg};
pub use error::AppHttpError;
pub use logging::{init_tracing, init_tracing_with_filter};
pub use proxy_layer::MinifluxProxyLayer;
pub use routes::build_axum_routes;
pub use state::AppState;
