//! Async repository structs over `sqlx::SqlitePool`.

use sqlx::SqlitePool;

use crate::error::{DbError, Result};
use crate::models::{Book, ReadingProgress};

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
