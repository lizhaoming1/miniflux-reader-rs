//! Sync feed documents into the DB, and a background poller.

use std::time::Duration;

use progress_db::{ArticleRepository, ArticleUpsert, FeedRepository};
use sqlx::SqlitePool;
use tracing::{error, info, warn};

use crate::parse::fetch_feed;
use crate::Result;

/// Runtime-overridable config passed to sync functions.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// HTTP timeout for feed fetches.
    pub fetch_timeout: Duration,
    /// User-Agent header.
    pub user_agent: Option<String>,
}

/// Fetch a single feed by id and upsert its articles.
///
/// Returns the number of newly inserted articles (not total upserts).
pub async fn sync_feed(pool: &SqlitePool, feed_id: i64, cfg: &SyncConfig) -> Result<usize> {
    let feeds = FeedRepository::new(pool.clone());
    let feed = feeds.get(feed_id).await?;
    let parsed = match fetch_feed(&feed.url, cfg.fetch_timeout, cfg.user_agent.as_deref()).await {
        Ok(p) => p,
        Err(e) => {
            // Persist the error so the UI can show it; ignore persistence failures.
            feeds
                .update_fetched(feed_id, Some(&e.to_string()))
                .await
                .ok();
            return Err(e);
        }
    };
    // Clear error + update metadata + set last_fetched.
    feeds.update_fetched(feed_id, None).await?;
    if !parsed.title.is_empty() || !parsed.site_url.is_empty() {
        feeds
            .update_metadata(feed_id, &parsed.title, &parsed.site_url)
            .await?;
    }
    let articles = ArticleRepository::new(pool.clone());
    let mut inserted = 0usize;
    for art in parsed.articles {
        let ups = ArticleUpsert {
            guid: art.guid,
            title: art.title,
            url: art.url,
            author: art.author,
            summary: art.summary,
            content_html: art.content_html,
            published_at: art.published_at.to_rfc3339(),
        };
        if articles.upsert(feed_id, &ups).await? {
            inserted += 1;
        }
    }
    Ok(inserted)
}

/// Walk every feed in the DB and sync it. Single feed failures are logged
/// and do not abort the remaining syncs.
///
/// Returns total newly inserted articles.
pub async fn sync_all_feeds(pool: &SqlitePool, cfg: &SyncConfig) -> usize {
    let feeds = match FeedRepository::new(pool.clone()).list().await {
        Ok(f) => f,
        Err(e) => {
            error!(err = %e, "sync_all_feeds: could not list feeds");
            return 0;
        }
    };
    let mut total = 0usize;
    for f in feeds {
        match sync_feed(pool, f.id, cfg).await {
            Ok(n) => {
                info!(feed_id = f.id, url = %f.url, new = n, "synced feed");
                total += n;
            }
            Err(e) => warn!(feed_id = f.id, url = %f.url, err = %e, "sync_feed failed"),
        }
    }
    total
}

/// Start an infinite loop task that calls [`sync_all_feeds`] on each tick.
///
/// First tick fires immediately so that cold-start DBs get an initial
/// fetch. Errors inside `sync_all_feeds` are caught and logged there.
pub async fn start_poller(pool: SqlitePool, interval: Duration, cfg: SyncConfig) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        let _tick = ticker.tick().await;
        let n = sync_all_feeds(&pool, &cfg).await;
        info!(new = n, "poller sync completed");
    }
}
