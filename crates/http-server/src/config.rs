//! Application configuration: structs matching `rust-config.example.json`
//! and a file loader. v0.2.0 replaced the Miniflux upstream config block
//! with a built-in feed-engine block.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use thiserror::Error;

/// Configuration for the built-in RSS / Atom feed engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedCfg {
    /// Interval between full syncs, in seconds.
    #[serde(default = "default_poll")]
    pub poll_interval_secs: u64,
    /// Per-request timeout for feed fetches / readability extraction, seconds.
    #[serde(default = "default_fetch_timeout")]
    pub fetch_timeout_secs: u64,
    /// User-Agent header sent when fetching feeds.
    #[serde(default = "default_ua")]
    pub user_agent: String,
}

fn default_poll() -> u64 {
    900
}
fn default_fetch_timeout() -> u64 {
    30
}
fn default_ua() -> String {
    "miniflux-reader-rs/0.2.0".to_string()
}

impl Default for FeedCfg {
    fn default() -> Self {
        Self {
            poll_interval_secs: default_poll(),
            fetch_timeout_secs: default_fetch_timeout(),
            user_agent: default_ua(),
        }
    }
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
    /// Directory containing the wasm `pkg/` bundle produced by `cargo-leptos`.
    /// Defaults to `pkg` relative to the working directory when omitted.
    pub pkg_dir: Option<String>,
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
    /// Target language for translation (runtime overridable via settings).
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
}

fn default_target_lang() -> String {
    "zh-CN".to_string()
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
    /// Built-in RSS / Atom feed engine settings.
    #[serde(default)]
    pub feed: FeedCfg,
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
