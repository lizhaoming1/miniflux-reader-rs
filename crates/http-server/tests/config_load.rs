//! PR#6 — `AppConfig` loader TDD: verifies the real config struct matches
//! `rust-config.example.json` and that `load()` parses all fields correctly.

use http_server::config::{load, AppConfig};
use serde_json::json;
use std::io::Write;

fn write_temp_config(content: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.keep().join("rust-config.json");
    let mut f = std::fs::File::create(&path).expect("create");
    f.write_all(content.as_bytes()).expect("write");
    path
}

#[test]
fn t0_load_full_config_parses_all_fields() {
    let cfg_json = json!({
        "listen_addr": "0.0.0.0:8083",
        "miniflux": {
            "url": "http://127.0.0.1:8081",
            "username": "admin",
            "password": "change-me"
        },
        "paths": {
            "db":            "rust-data/epub_progress_rust.db",
            "epub_dir":      "rust-epub-books",
            "static_inject": "crates/http-server/assets"
        },
        "translate": {
            "endpoint":     "https://translate.googleapis.com/translate_a/single",
            "timeout_ms":   15000,
            "batch_size":   25,
            "max_retries":  3
        },
        "tts": {
            "voice":        "zh-CN-XiaoxiaoNeural",
            "rate":         "+0%",
            "timeout_ms":   60000
        },
        "log_filter": "info,http_server=debug"
    })
    .to_string();

    let path = write_temp_config(&cfg_json);
    let cfg: AppConfig = load(path.to_str().unwrap()).expect("load should succeed");

    assert_eq!(cfg.listen_addr, "0.0.0.0:8083");
    assert_eq!(cfg.miniflux.url, "http://127.0.0.1:8081");
    assert_eq!(cfg.miniflux.username, "admin");
    assert_eq!(cfg.miniflux.password, "change-me");
    assert_eq!(cfg.paths.db, "rust-data/epub_progress_rust.db");
    assert_eq!(cfg.paths.epub_dir, "rust-epub-books");
    assert_eq!(cfg.paths.static_inject, "crates/http-server/assets");
    assert_eq!(
        cfg.translate.endpoint,
        "https://translate.googleapis.com/translate_a/single"
    );
    assert_eq!(cfg.translate.timeout_ms, 15000);
    assert_eq!(cfg.translate.batch_size, 25);
    assert_eq!(cfg.translate.max_retries, 3);
    assert_eq!(cfg.tts.voice, "zh-CN-XiaoxiaoNeural");
    assert_eq!(cfg.tts.rate, "+0%");
    assert_eq!(cfg.tts.timeout_ms, 60000);
    assert_eq!(cfg.log_filter, "info,http_server=debug");
}

#[test]
fn t1_load_missing_file_returns_error() {
    let result: Result<AppConfig, _> = load("/nonexistent/path/config.json");
    assert!(result.is_err());
}

#[test]
fn t2_load_malformed_json_returns_error() {
    let path = write_temp_config("{ this is not json }");
    let result: Result<AppConfig, _> = load(path.to_str().unwrap());
    assert!(result.is_err());
}
