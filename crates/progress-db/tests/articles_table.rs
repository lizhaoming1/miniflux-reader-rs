use progress_db::{
    run_migrations, ArticleRepository, ArticleUpsert, FeedRepository,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tempfile::TempDir;

async fn fresh_pool() -> (SqlitePool, TempDir) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("t.db").to_string_lossy().to_string();
    let url = format!("sqlite://{}", path);
    let opts = SqliteConnectOptions::from_str(&url).unwrap().create_if_missing(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (pool, tmp)
}

fn upsert(guid: &str, title: &str) -> ArticleUpsert {
    ArticleUpsert {
        guid: guid.into(),
        title: title.into(),
        url: format!("https://example.com/{}", guid),
        author: Some("me".into()),
        summary: "summary".into(),
        content_html: Some("<p>body</p>".into()),
        published_at: "2026-07-30T00:00:00+00:00".into(),
    }
}

#[tokio::test]
async fn t01_upsert_dedupes_on_guid() {
    let (pool, _t) = fresh_pool().await;
    let feeds = FeedRepository::new(pool.clone());
    let f = feeds.add("https://f1.example.com").await.unwrap();
    let arts = ArticleRepository::new(pool.clone());
    let inserted1 = arts.upsert(f.id, &upsert("g1", "A")).await.unwrap();
    let inserted2 = arts.upsert(f.id, &upsert("g1", "A — updated")).await.unwrap();
    assert!(inserted1);
    assert!(!inserted2); // second upsert matches, no new row
    let list = arts.list_all(Some(f.id), 100, 0).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "A — updated");
}

#[tokio::test]
async fn t02_list_unread_filters_read() {
    let (pool, _t) = fresh_pool().await;
    let feeds = FeedRepository::new(pool.clone());
    let f = feeds.add("https://f2.example.com").await.unwrap();
    let arts = ArticleRepository::new(pool.clone());
    arts.upsert(f.id, &upsert("g1", "A")).await.unwrap();
    arts.upsert(f.id, &upsert("g2", "B")).await.unwrap();
    let a1 = arts.list_all(Some(f.id), 10, 0).await.unwrap().remove(0);
    arts.set_read(a1.id, true).await.unwrap();
    let unread = arts.list_unread(Some(f.id), 100).await.unwrap();
    assert_eq!(unread.len(), 1);
}

#[tokio::test]
async fn t03_set_read_roundtrip() {
    let (pool, _t) = fresh_pool().await;
    let feeds = FeedRepository::new(pool.clone());
    let f = feeds.add("https://f3.example.com").await.unwrap();
    let arts = ArticleRepository::new(pool.clone());
    arts.upsert(f.id, &upsert("g1", "A")).await.unwrap();
    let a = arts.list_all(Some(f.id), 10, 0).await.unwrap().remove(0);
    assert!(!a.is_read);
    arts.set_read(a.id, true).await.unwrap();
    let got = arts.get(a.id).await.unwrap();
    assert!(got.is_read);
}

#[tokio::test]
async fn t04_mark_all_read_single_feed() {
    let (pool, _t) = fresh_pool().await;
    let feeds = FeedRepository::new(pool.clone());
    let f1 = feeds.add("https://f4a.example.com").await.unwrap();
    let f2 = feeds.add("https://f4b.example.com").await.unwrap();
    let arts = ArticleRepository::new(pool.clone());
    for i in 1..=3 {
        arts.upsert(f1.id, &upsert(&format!("g-f1-{i}"), &format!("F1-{i}"))).await.unwrap();
        arts.upsert(f2.id, &upsert(&format!("g-f2-{i}"), &format!("F2-{i}"))).await.unwrap();
    }
    arts.mark_all_read(Some(f1.id)).await.unwrap();
    let count = arts.count_unread().await.unwrap();
    assert_eq!(count, 3); // only f2 still unread
}

#[tokio::test]
async fn t05_count_unread_on_empty_is_zero() {
    let (pool, _t) = fresh_pool().await;
    let arts = ArticleRepository::new(pool);
    assert_eq!(arts.count_unread().await.unwrap(), 0);
}
