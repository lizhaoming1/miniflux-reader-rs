-- ==========================================================================
-- 008_create_articles.sql
-- Articles fetched from RSS / Atom feeds. UNIQUE(feed_id, guid) dedupes
-- re-fetches. ON DELETE CASCADE from feeds drops articles on feed removal.
-- ==========================================================================
CREATE TABLE IF NOT EXISTS articles (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id       INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    guid          TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    url           TEXT NOT NULL DEFAULT '',
    author        TEXT,
    summary       TEXT NOT NULL DEFAULT '',
    content_html  TEXT,
    published_at  TEXT NOT NULL,
    read          INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(feed_id, guid)
);
CREATE INDEX IF NOT EXISTS idx_articles_feed_id ON articles(feed_id);
CREATE INDEX IF NOT EXISTS idx_articles_published ON articles(published_at DESC);
