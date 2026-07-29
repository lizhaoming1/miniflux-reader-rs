//! # http-server
//!
//! The single executable crate that wires everything together.
//! Exported as a library (`http_server`) **and** a binary to keep the
//! Router testable via `axum::body::Body` + `Router::oneshot`.
//! 
//! Route priority (match order is significant because of the catch-all
//! miniflux proxy):
//!   1. Static / static assets and well-known axum routes
//!      (login, upload, tts, translate)
//!   2. Leptos `Router` — named pages (/epub, /epub/read/:safe, …)
//!   3. Fallback `MinifluxProxyLayer` as a tower service — *everything*
//!      that didn't match above is forwarded to `config.miniflux.url`.

#![forbid(unsafe_code)]

pub mod config;
pub mod state;
pub mod routes;
pub mod proxy_layer;
pub mod error;
pub mod logging;

pub use config::{AppConfig, MinifluxCfg, LoadCfgError};
pub use state::AppState;
pub use routes::build_axum_routes;
pub use proxy_layer::MinifluxProxyLayer;
pub use logging::init_tracing;
