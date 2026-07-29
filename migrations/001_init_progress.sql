-- ==========================================================================
-- 001_init_progress.sql
-- Keyed on `epub_path` (the rust-epub-books/<safe_name>.epub path).
-- Schema is DELIBERATELY NOT aligned with the Python reading-platform
-- project's data/epub_progress.db; per Plan C we run isolated stacks.
-- ==========================================================================
CREATE TABLE IF NOT EXISTS reading_progress (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    epub_path     TEXT    NOT NULL UNIQUE,
    chapter_idx   INTEGER NOT NULL DEFAULT 0,
    scroll_pos    INTEGER NOT NULL DEFAULT 0,
    percent       REAL    NOT NULL DEFAULT 0.0,
    overall       REAL    NOT NULL DEFAULT 0.0,
    updated_at    TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_progress_epub_path ON reading_progress(epub_path);
