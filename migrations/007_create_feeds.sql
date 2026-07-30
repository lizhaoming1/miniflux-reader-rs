-- ==========================================================================
-- 007_create_feeds.sql
-- RSS / Atom feed subscriptions.
-- ==========================================================================
CREATE TABLE IF NOT EXISTS feeds (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    url           TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL DEFAULT '',
    site_url      TEXT NOT NULL DEFAULT '',
    last_fetched  TEXT,
    fetch_error   TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
