use progress_db::{run_migrations, SettingsRepository};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tempfile::TempDir;

async fn fresh_pool() -> (SqlitePool, TempDir) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("t.db").to_string_lossy().to_string();
    let url = format!("sqlite://{}", path);
    let opts = SqliteConnectOptions::from_str(&url)
        .unwrap()
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    (pool, tmp)
}

#[tokio::test]
async fn t01_get_all_empty() {
    let (pool, _t) = fresh_pool().await;
    let repo = SettingsRepository::new(pool);
    let all = repo.get_all().await.unwrap();
    assert!(all.is_empty());
}

#[tokio::test]
async fn t02_set_many_then_get_all() {
    let (pool, _t) = fresh_pool().await;
    let repo = SettingsRepository::new(pool);
    let mut in_map = std::collections::HashMap::new();
    in_map.insert("feed.poll_interval_secs".into(), "1200".into());
    in_map.insert("tts.voice".into(), "custom-voice".into());
    repo.set_many(&in_map).await.unwrap();
    let out = repo.get_all().await.unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(
        out.get("feed.poll_interval_secs"),
        Some(&"1200".to_string())
    );
    assert_eq!(out.get("tts.voice"), Some(&"custom-voice".to_string()));
}

#[tokio::test]
async fn t03_get_single_missing_returns_none() {
    let (pool, _t) = fresh_pool().await;
    let repo = SettingsRepository::new(pool);
    let got = repo.get("nonexistent").await.unwrap();
    assert!(got.is_none());
}
