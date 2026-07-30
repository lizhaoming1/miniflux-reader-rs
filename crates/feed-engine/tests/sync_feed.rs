use feed_engine::parse::parse_feed_bytes;
use feed_engine::sync::{sync_feed, SyncConfig};
use progress_db::{run_migrations, ArticleRepository, FeedRepository};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn fresh_pool() -> (SqlitePool, TempDir) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("t.db").to_string_lossy().to_string();
    let url = format!("sqlite://{}", path);
    let opts = SqliteConnectOptions::from_str(&url).unwrap().create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    (pool, tmp)
}

fn rss_body() -> &'static [u8] {
    br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Mock RSS</title>
    <link>https://mock.example/</link>
    <item>
      <guid>m1</guid><title>M1</title>
      <link>https://mock.example/1</link>
      <pubDate>Thu, 30 Jul 2026 00:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>"#
}

#[tokio::test]
async fn t01_sync_feed_inserts_articles_updates_metadata() {
    let (pool, _t) = fresh_pool().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rss"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(rss_body().to_vec(), "application/rss+xml"),
        )
        .mount(&server)
        .await;
    let url = format!("{}/rss", server.uri());

    let feeds = FeedRepository::new(pool.clone());
    let f = feeds.add(&url).await.unwrap();
    let cfg = SyncConfig {
        fetch_timeout: Duration::from_secs(10),
        user_agent: Some("tester".into()),
    };
    let n = sync_feed(&pool, f.id, &cfg).await.unwrap();
    assert_eq!(n, 1);

    let f2 = feeds.get(f.id).await.unwrap();
    assert_eq!(f2.title, "Mock RSS");
    assert_eq!(f2.site_url, "https://mock.example/");
    assert!(f2.last_fetched.is_some());
    assert!(f2.fetch_error.is_none());

    let arts = ArticleRepository::new(pool);
    let list = arts.list_all(Some(f.id), 10, 0).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "M1");
    let _ = parse_feed_bytes(rss_body()).unwrap(); // covers unused import
}

#[tokio::test]
async fn t02_sync_feed_upstream_500_writes_fetch_error() {
    let (pool, _t) = fresh_pool().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/bad"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let url = format!("{}/bad", server.uri());
    let feeds = FeedRepository::new(pool.clone());
    let f = feeds.add(&url).await.unwrap();
    let cfg = SyncConfig {
        fetch_timeout: Duration::from_secs(3),
        user_agent: None,
    };
    let err = sync_feed(&pool, f.id, &cfg).await.expect_err("should fail");
    assert!(matches!(err, feed_engine::FeedEngineError::UpstreamStatus(500)));
    let f2 = feeds.get(f.id).await.unwrap();
    assert!(f2.fetch_error.is_some(), "fetch_error should be set");
}
