//! Application configuration: structs matching `rust-config.example.json`
//! and a file loader.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use thiserror::Error;

/// Miniflux upstream connection settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MinifluxCfg {
    /// Base URL of the Miniflux server.
    pub url: String,
    /// Miniflux account username.
    pub username: String,
    /// Miniflux account password.
    pub password: String,
}

/// Filesystem paths for data isolation (Plan C: `rust-*` prefix).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathsCfg {
    /// SQLite database path (e.g. `rust-data/epub_progress_rust.db`).
    pub db: String,
    /// Directory for uploaded EPUB files.
    pub epub_dir: String,
    /// Directory for static inject assets (e.g. `_inject_rs.js`).
    pub static_inject: String,
}

/// Translation backend settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranslateCfg {
    /// Upstream translate API endpoint.
    pub endpoint: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum segments per batch.
    pub batch_size: usize,
    /// Maximum retry attempts on transient failure.
    pub max_retries: u32,
}

/// TTS backend settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TtsCfg {
    /// Voice identifier (e.g. `zh-CN-XiaoxiaoNeural`).
    pub voice: String,
    /// Speech rate adjustment (e.g. `+0%`).
    pub rate: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

/// Top-level application configuration, deserialised from
/// `rust-config.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Socket address to listen on (e.g. `0.0.0.0:8083`).
    pub listen_addr: String,
    /// Miniflux upstream settings.
    pub miniflux: MinifluxCfg,
    /// Filesystem paths.
    pub paths: PathsCfg,
    /// Translation backend settings.
    pub translate: TranslateCfg,
    /// TTS backend settings.
    pub tts: TtsCfg,
    /// tracing env-filter string.
    pub log_filter: String,
}

/// Errors that can occur while loading configuration.
#[derive(Debug, Error)]
pub enum LoadCfgError {
    /// Failed to read the config file from disk.
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to parse the config JSON.
    #[error("failed to parse config JSON: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Load and parse the configuration file at `path`.
pub fn load(path: &str) -> Result<AppConfig, LoadCfgError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let cfg: AppConfig = serde_json::from_reader(reader)?;
    Ok(cfg)
}
