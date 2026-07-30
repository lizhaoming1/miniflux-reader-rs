use progress_db::{run_migrations, FeedRepository};
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
async fn t01_add_and_get() {
    let (pool, _t) = fresh_pool().await;
    let repo = FeedRepository::new(pool);
    let f = repo.add("https://example.com/feed.xml").await.unwrap();
    assert_eq!(f.url, "https://example.com/feed.xml");
    let got = repo.get(f.id).await.unwrap();
    assert_eq!(got.url, f.url);
}

#[tokio::test]
async fn t02_add_same_url_is_idempotent() {
    let (pool, _t) = fresh_pool().await;
    let repo = FeedRepository::new(pool);
    let f1 = repo.add("https://example.com/dup.xml").await.unwrap();
    let f2 = repo.add("https://example.com/dup.xml").await.unwrap();
    assert_eq!(f1.id, f2.id);
}

#[tokio::test]
async fn t03_list_ordered_by_title() {
    let (pool, _t) = fresh_pool().await;
    let repo = FeedRepository::new(pool.clone());
    let fa = repo.add("https://a.example.com").await.unwrap();
    let fb = repo.add("https://b.example.com").await.unwrap();
    repo.update_metadata(fa.id, "Alpha", "").await.unwrap();
    repo.update_metadata(fb.id, "Beta", "").await.unwrap();
    let list = repo.list().await.unwrap();
    assert_eq!(list[0].title, "Alpha");
    assert_eq!(list[1].title, "Beta");
}

#[tokio::test]
async fn t04_remove_cascades_invalidates_get() {
    let (pool, _t) = fresh_pool().await;
    let repo = FeedRepository::new(pool);
    let f = repo.add("https://del.example.com").await.unwrap();
    repo.remove(f.id).await.unwrap();
    let err = repo.get(f.id).await.expect_err("should be not found");
    assert!(matches!(err, progress_db::DbError::NotFound(_)));
}
