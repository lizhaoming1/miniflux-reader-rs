-- ==========================================================================
-- 009_create_settings.sql
-- Runtime-overridable settings as key/value. Only keys listed in the spec
-- are written; others are ignored on GET / PUT /settings routes.
-- ==========================================================================
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
