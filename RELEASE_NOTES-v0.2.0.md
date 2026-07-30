# Release v0.2.0 — Built-in RSS Engine, Miniflux Dependency Removed

**Date:** 2026-07-30
**Tag:** `v0.2.0`
**Branch:** `rust` → `main`

## Headline

Transforms `miniflux-reader-rs` from a Miniflux proxy frontend into a
self-contained unified reading platform. The Miniflux server dependency is
**fully removed** — RSS/Atom feed parsing, article fetching, scheduled polling,
OPML import/export, and a runtime settings UI are now built in. EPUB reading,
bilingual translation injection, and TTS synthesis are preserved unchanged.

Single-binary deployment still works: one `http-server` binary serves the
Leptos SSR UI, the RSS/EPUB HTTP API, the background feed poller, and the
translate/TTS proxies.

## What's new

### 1. New crate: `feed-engine`

RSS/Atom engine implemented from scratch on top of `feed-rs` + `readability`:

| Module | Responsibility |
|--------|----------------|
| `parse.rs` | `ParsedFeed` / `ParsedArticle` models + `fetch_feed()` HTTP fetcher |
| `sync.rs` | `sync_feed()` single-feed sync, `sync_all_feeds()`, `start_poller()` background loop |
| `opml.rs` | `parse_opml()` import + `export_opml()` export (quick-xml) |
| `content.rs` | `extract_content()` readability-based article body extraction |
| `lib.rs` | `FeedEngineError` enum + `Result` alias |

### 2. New database tables (migrations 007–009)

| Migration | Table | Key constraints |
|-----------|-------|-----------------|
| `007_create_feeds.sql` | `feeds` | `url UNIQUE`, `last_fetched`, `fetch_error` |
| `008_create_articles.sql` | `articles` | `UNIQUE(feed_id, guid)` dedup, `ON DELETE CASCADE`, indexes on `feed_id` + `published_at DESC` |
| `009_create_settings.sql` | `settings` | `key PRIMARY KEY` / `value TEXT` |

Migration version is now **9** (was 6 in v0.1.0).

### 3. Three new repositories in `progress-db`

- `FeedRepository` — add / list / get / remove + `update_fetch_status()`
- `ArticleRepository` — upsert (with pre-check insert detection) / list /
  mark_read / mark_all_read / count_unread
- `SettingsRepository` — get / set_many / get_all

### 4. Refactored `http-server`

**Removed:**
- `proxy_layer.rs` (catch-all Miniflux proxy + CSP stripping)
- `mf-login` route + `MinifluxCfg` config block
- `services::miniflux_client` module + tests
- `scraper` workspace dependency (only used by the proxy)

**Added (14 new routes):**

| Route | Method | Handler |
|-------|--------|---------|
| `/feeds` | GET | `list_feeds` |
| `/feeds` | POST | `add_feed` |
| `/feeds/:id` | DELETE | `remove_feed` |
| `/feeds/sync` | POST | `sync_all_handler` |
| `/articles` | GET | `list_articles` |
| `/articles/:id` | GET | `get_article` |
| `/articles/:id/read` | POST | `set_article_read` |
| `/articles/read-all` | POST | `mark_all_read` |
| `/articles/unread-count` | GET | `unread_count` |
| `/opml/import` | POST | `opml_import` |
| `/opml/export` | GET | `opml_export` |
| `/settings` | GET | `get_settings` |
| `/settings` | PUT | `update_settings` |
| `/feeds/sync` (background) | — | `start_poller()` spawned in `main.rs` |

Config block renamed `miniflux` → `feed` with fields `poll_interval_secs`,
`fetch_timeout_secs`, `user_agent`.

### 5. Refactored `leptos-app`

- `MinifluxArticle` component → renamed `ArticleReader` (reused for RSS articles)
- New components: `FeedList`, `AddFeedForm`, `ArticleList`, `OpmlActions`, `SettingsPage`
- `App` router wires 6 routes: `/`, `/feeds`, `/article/:id`, `/settings`,
  `/epub`, `/epub/read/:name`
- Top-bar nav: Articles · Feeds · EPUB · Settings

### 6. Runtime settings UI

`SettingsPage` exposes the previously-compile-time config keys at runtime:

| Key | Default | Purpose |
|-----|---------|---------|
| `feed.poll_interval_secs` | `900` | Background poller interval |
| `feed.fetch_timeout_secs` | `30` | Per-feed HTTP timeout |
| `feed.user_agent` | `miniflux-reader-rs/0.2.0` | User-Agent header |
| `translate.target_lang` | `zh-CN` | Bilingual translation target |
| `tts.voice` | `zh-CN-XiaoxiaoNeural` | Edge-TTS voice |
| `tts.rate` | `+0%` | TTS speaking rate |

Settings persist in the `settings` table; the poller reads them on each tick.

## Test stats

- **116 tests pass** (was 104 in v0.1.0)
- **4 tests ignored** (unchanged — `wasm-pack` + headless Chrome tests)
- 0 fmt warnings, 0 clippy warnings (`-D warnings`)
- 9/9 SQLite migrations succeed (version=9)

### New test coverage

| Crate | New tests | File(s) |
|-------|-----------|---------|
| `progress-db` | 12 | `tests/feeds_table.rs`, `tests/articles_table.rs`, `tests/settings_table.rs` |
| `feed-engine` | 7 | `tests/feed_parse.rs`, `tests/sync_feed.rs`, `tests/opml.rs` |
| `http-server` | 12 | `tests/feed_routes.rs`, `tests/article_routes.rs`, `tests/opml_routes.rs`, `tests/settings_routes.rs` |
| `leptos-app` | 5 | `tests/ssr_render.rs` (FeedList / ArticleList / AddFeedForm / SettingsPage / OpmlActions) |

## Build & run

```bash
# From source
cargo install cargo-leptos
cp rust-config.example.json rust-config.json   # edit feed/translate/tts blocks
cargo leptos watch          # dev: SSR + hot reload on :3000
cargo leptos build --release && ./target/release/http-server rust-config.json
                             # prod: :8083

# Docker
docker build -t miniflux-reader-rs:v0.2.0 .
docker run -p 8083:8083 \
  -v $(pwd)/rust-config.json:/app/rust-config.json \
  -v $(pwd)/rust-data:/app/rust-data \
  -v $(pwd)/rust-epub-books:/app/rust-epub-books \
  miniflux-reader-rs:v0.2.0
```

No `miniflux.url` / `username` / `password` config keys anymore — just point
your browser at `:8083`, open the Feeds page, and paste an RSS/Atom URL.

## Migration guide (v0.1.0 → v0.2.0)

1. **Stop the v0.1.0 server.**
2. **Back up the DB** (always, before a major version bump):
   `cp rust-data/epub_progress_rust.db rust-data/epub_progress_rust.db.v010-backup`
3. **Drop the new binary in place.** Migrations 007–009 run automatically on
   first boot and are additive only (no schema changes to existing tables).
4. **Update `rust-config.json`:** remove the `miniflux` block, add a `feed`
   block (see `rust-config.example.json`).
5. **(Optional) Import existing OPML:** open `/feeds` in the UI → "Import OPML".

The EPUB bookshelf and reading progress are untouched — v0.1.0 users keep
their books and progress.

## What was removed (breaking)

| Removed | Replacement |
|---------|-------------|
| `services::miniflux_client` | `feed-engine` crate |
| `proxy_layer.rs` (catch-all proxy) | 14 explicit routes in `routes.rs` |
| `/mf-login` route | Not needed — no upstream auth |
| `MinifluxCfg` config block | `FeedCfg` (`poll_interval_secs` / `fetch_timeout_secs` / `user_agent`) |
| `MF_SESSION_RS` cookie | Not needed |
| `scraper` workspace dep | Removed (was only used by proxy) |

## Known limitations

- **No auth on the Rust server itself** (carried over from v0.1.0 §YAGNI-1).
  Single-user self-hosted assumption still holds.
- **No feed discovery / auto-complete** — you must paste the exact feed URL.
  Future enhancement: parse `<link rel="alternate" type="application/rss+xml">`
  from a site URL.
- **No article search / full-text index** — list view filters by feed + read
  state only. FTS deferred.
- **Poller is in-process** — restart kills in-flight fetches. Acceptable for
  single-binary MVP; durable scheduler deferred.

## Verification evidence

```
cargo fmt --all -- --check                              → 0 warnings
cargo clippy --workspace --all-targets -- -D warnings   → 0 errors
cargo test --workspace
  → 116 passed / 0 failed / 4 ignored
sqlx migrate run (fresh SQLite)                         → version=9, 9/9 success
  → feeds / articles / settings tables created empty
```

## What's next (v0.3.0 candidates)

Not committed — just tracked:
- Feed auto-discovery from a site URL
- Article full-text search (SQLite FTS5)
- Per-feed fetch interval override
- Article star/favorite + custom tags
- Reading-time estimate per article
- Prometheus metrics endpoint
