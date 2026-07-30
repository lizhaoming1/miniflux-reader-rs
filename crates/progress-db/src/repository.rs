//! Async repository structs over `sqlx::SqlitePool`.

use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::error::{DbError, Result};
use crate::models::{Article, Book, Feed, ReadingProgress};

/// Repository for the `reading_progress` table.
#[derive(Clone)]
pub struct ProgressRepository {
    pool: SqlitePool,
}

impl ProgressRepository {
    /// Create a new repository bound to `pool`.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Save (upsert) a `ReadingProgress` row. If `epub_path` already
    /// exists, all fields are updated.
    pub async fn save(&self, p: &ReadingProgress) -> Result<()> {
        sqlx::query(
            "INSERT INTO reading_progress (epub_path, chapter_idx, scroll_pos, percent, overall, updated_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(epub_path) DO UPDATE SET
               chapter_idx = excluded.chapter_idx,
               scroll_pos  = excluded.scroll_pos,
               percent     = excluded.percent,
               overall     = excluded.overall,
               updated_at  = datetime('now')",
        )
        .bind(&p.epub_path)
        .bind(p.chapter_idx)
        .bind(p.scroll_pos)
        .bind(p.percent)
        .bind(p.overall)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get the progress row for `epub_path`. Returns `NotFound` if absent.
    pub async fn get(&self, epub_path: &str) -> Result<ReadingProgress> {
        let row = sqlx::query_as::<_, ReadingProgress>(
            "SELECT epub_path, chapter_idx, scroll_pos, percent, overall
             FROM reading_progress WHERE epub_path = ?",
        )
        .bind(epub_path)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| DbError::NotFound(epub_path.to_string()))
    }
}

/// Repository for the `books` table.
#[derive(Clone)]
pub struct BookRepository {
    pool: SqlitePool,
}

impl BookRepository {
    /// Create a new repository bound to `pool`.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert or update a book row.
    pub async fn upsert(&self, b: &Book) -> Result<()> {
        sqlx::query(
            "INSERT INTO books (safe_name, title, author, total_chapters, file_size, created_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(safe_name) DO UPDATE SET
               title = excluded.title,
               author = excluded.author,
               total_chapters = excluded.total_chapters,
               file_size = excluded.file_size",
        )
        .bind(&b.safe_name)
        .bind(&b.title)
        .bind(&b.author)
        .bind(b.total_chapters)
        .bind(b.file_size)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// List all books ordered by `created_at DESC`.
    pub async fn list(&self) -> Result<Vec<Book>> {
        let rows = sqlx::query_as::<_, Book>(
            "SELECT safe_name, title, author, total_chapters, file_size
             FROM books ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------------
// FeedRepository
// ---------------------------------------------------------------------

/// Repository for the `feeds` table.
#[derive(Clone)]
pub struct FeedRepository {
    pool: SqlitePool,
}

impl FeedRepository {
    /// Create a new repository bound to `pool`.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new feed by `url`. If `url` already exists returns the
    /// existing row. Title/site_url are left as `''` on new-insert and
    /// populated on first successful sync.
    pub async fn add(&self, url: &str) -> Result<Feed> {
        // Try insert first.
        let insert = sqlx::query(
            "INSERT OR IGNORE INTO feeds (url) VALUES (?)",
        )
        .bind(url)
        .execute(&self.pool)
        .await?;
        let _ = insert;
        self.get_by_url(url).await
    }

    /// Fetch feed by primary key. Returns `NotFound` if absent.
    pub async fn get(&self, id: i64) -> Result<Feed> {
        let row = sqlx::query_as::<_, Feed>(
            "SELECT id, url, title, site_url, last_fetched, fetch_error
             FROM feeds WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| DbError::NotFound(format!("feed id={id}")))
    }

    /// Fetch feed by URL. Returns `NotFound` if absent.
    pub async fn get_by_url(&self, url: &str) -> Result<Feed> {
        let row = sqlx::query_as::<_, Feed>(
            "SELECT id, url, title, site_url, last_fetched, fetch_error
             FROM feeds WHERE url = ?",
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| DbError::NotFound(format!("feed url={url}")))
    }

    /// List all feeds ordered by title ASC (empty titles after).
    pub async fn list(&self) -> Result<Vec<Feed>> {
        let rows = sqlx::query_as::<_, Feed>(
            "SELECT id, url, title, site_url, last_fetched, fetch_error
             FROM feeds ORDER BY (title = ''), title ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Delete a feed by id. Articles are removed by ON DELETE CASCADE.
    pub async fn remove(&self, id: i64) -> Result<()> {
        let r = sqlx::query("DELETE FROM feeds WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("feed id={id}")));
        }
        Ok(())
    }

    /// Update `last_fetched` to now and optionally set `fetch_error`
    /// (None clears any previous error).
    pub async fn update_fetched(&self, id: i64, error: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE feeds SET
                last_fetched = datetime('now'),
                fetch_error  = ?
             WHERE id = ?",
        )
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update title and site_url (used after a successful parse).
    pub async fn update_metadata(&self, id: i64, title: &str, site_url: &str) -> Result<()> {
        sqlx::query(
            "UPDATE feeds SET title = ?, site_url = ? WHERE id = ?",
        )
        .bind(title)
        .bind(site_url)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------
// ArticleRepository
// ---------------------------------------------------------------------

/// Repository for the `articles` table.
#[derive(Clone)]
pub struct ArticleRepository {
    pool: SqlitePool,
}

impl ArticleRepository {
    /// Create a new repository bound to `pool`.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Upsert an article row keyed on (feed_id, guid). All scalar fields
    /// are updated on conflict. Returns `true` if a new row was inserted.
    pub async fn upsert(&self, feed_id: i64, a: &ArticleUpsert) -> Result<bool> {
        // SQLite's last_insert_rowid() is not reset on the ON CONFLICT
        // DO UPDATE path, so it cannot distinguish insert from update.
        // A pre-check is the simplest reliable approach.
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM articles WHERE feed_id = ? AND guid = ?",
        )
        .bind(feed_id)
        .bind(&a.guid)
        .fetch_optional(&self.pool)
        .await?;
        let is_new = existing.is_none();

        sqlx::query(
            "INSERT INTO articles (feed_id, guid, title, url, author, summary, content_html, published_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(feed_id, guid) DO UPDATE SET
                title = excluded.title,
                url   = excluded.url,
                author = excluded.author,
                summary = excluded.summary,
                content_html = excluded.content_html,
                published_at = excluded.published_at",
        )
        .bind(feed_id)
        .bind(&a.guid)
        .bind(&a.title)
        .bind(&a.url)
        .bind(&a.author)
        .bind(&a.summary)
        .bind(&a.content_html)
        .bind(&a.published_at)
        .execute(&self.pool)
        .await?;
        Ok(is_new)
    }

    /// Return all articles, newest first, paginated. `feed_id = None`
    /// lists across all feeds.
    pub async fn list_all(&self, feed_id: Option<i64>, limit: u32, offset: u32) -> Result<Vec<Article>> {
        let sql = match feed_id {
            Some(_) => "SELECT a.id, a.feed_id, a.guid, a.title, a.url, a.author, a.summary, a.content_html, a.published_at, a.read
                        FROM articles a WHERE a.feed_id = ?
                        ORDER BY a.published_at DESC LIMIT ? OFFSET ?",
            None => "SELECT a.id, a.feed_id, a.guid, a.title, a.url, a.author, a.summary, a.content_html, a.published_at, a.read
                     FROM articles a
                     ORDER BY a.published_at DESC LIMIT ? OFFSET ?",
        };
        let mut q = sqlx::query_as::<_, Article>(sql);
        if let Some(fid) = feed_id {
            q = q.bind(fid);
        }
        let rows: Vec<Article> = q
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    /// Return only unread articles, newest first. `feed_id = None` lists
    /// across all feeds.
    pub async fn list_unread(&self, feed_id: Option<i64>, limit: u32) -> Result<Vec<Article>> {
        let sql = match feed_id {
            Some(_) => "SELECT a.id, a.feed_id, a.guid, a.title, a.url, a.author, a.summary, a.content_html, a.published_at, a.read
                        FROM articles a WHERE a.feed_id = ? AND a.read = 0
                        ORDER BY a.published_at DESC LIMIT ?",
            None => "SELECT a.id, a.feed_id, a.guid, a.title, a.url, a.author, a.summary, a.content_html, a.published_at, a.read
                     FROM articles a WHERE a.read = 0
                     ORDER BY a.published_at DESC LIMIT ?",
        };
        let mut q = sqlx::query_as::<_, Article>(sql);
        if let Some(fid) = feed_id {
            q = q.bind(fid);
        }
        let rows: Vec<Article> = q.bind(limit as i64).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    /// Fetch an article by id, 404 if absent.
    pub async fn get(&self, id: i64) -> Result<Article> {
        let row = sqlx::query_as::<_, Article>(
            "SELECT a.id, a.feed_id, a.guid, a.title, a.url, a.author, a.summary, a.content_html, a.published_at, a.read
             FROM articles a WHERE a.id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| DbError::NotFound(format!("article id={id}")))
    }

    /// Mark article as read / unread.
    pub async fn set_read(&self, id: i64, read: bool) -> Result<()> {
        let r = sqlx::query("UPDATE articles SET read = ? WHERE id = ?")
            .bind(read)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if r.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("article id={id}")));
        }
        Ok(())
    }

    /// Mark all articles read. `feed_id = None` means every article.
    pub async fn mark_all_read(&self, feed_id: Option<i64>) -> Result<()> {
        match feed_id {
            Some(fid) => sqlx::query("UPDATE articles SET read = 1 WHERE feed_id = ?").bind(fid).execute(&self.pool).await?,
            None => sqlx::query("UPDATE articles SET read = 1").execute(&self.pool).await?,
        };
        Ok(())
    }

    /// Count of articles with read = 0.
    pub async fn count_unread(&self) -> Result<i64> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles WHERE read = 0")
            .fetch_one(&self.pool)
            .await?;
        Ok(c)
    }
}

/// Upsert fields for an article (used by ArticleRepository::upsert).
#[derive(Debug, Clone)]
pub struct ArticleUpsert {
    /// Feed-scoped unique id.
    pub guid: String,
    /// Title.
    pub title: String,
    /// Original article URL.
    pub url: String,
    /// Author, if any.
    pub author: Option<String>,
    /// Short summary.
    pub summary: String,
    /// Full HTML content, if any.
    pub content_html: Option<String>,
    /// ISO-8601 publication timestamp.
    pub published_at: String,
}

// ---------------------------------------------------------------------
// SettingsRepository
// ---------------------------------------------------------------------

/// Repository for the key/value `settings` table.
#[derive(Clone)]
pub struct SettingsRepository {
    pool: SqlitePool,
}

impl SettingsRepository {
    /// Create a new repository bound to `pool`.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return every (key, value) pair. Empty table returns empty map.
    pub async fn get_all(&self) -> Result<HashMap<String, String>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT key, value FROM settings")
                .fetch_all(&self.pool)
                .await?;
        let mut m = HashMap::with_capacity(rows.len());
        for (k, v) in rows {
            m.insert(k, v);
        }
        Ok(m)
    }

    /// Return a single setting or `None` if absent.
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM settings WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| r.0))
    }

    /// Bulk upsert of key/value pairs (wrapped in a single transaction).
    pub async fn set_many(&self, entries: &HashMap<String, String>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for (k, v) in entries {
            sqlx::query(
                "INSERT INTO settings (key, value) VALUES (?, ?)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )
            .bind(k)
            .bind(v)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
