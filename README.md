# miniflux-reader-rs 🔩

> Unified self-hosted reading platform — EPUB + RSS/Atom with bilingual
> translation injection and TTS synthesis. Rust + Axum 0.7 + Leptos 0.7 SSR +
> Hydration, single-binary deploy. No external feed server required.

## Scope

A single binary that combines:

| # | Capability | Crates touched |
|---|------------|----------------|
| R1 | RSS/Atom feed subscriptions + background polling | feed-engine / progress-db / http-server |
| R2 | Article list + reader with bilingual translation | feed-engine / common-text / services / leptos-app |
| R3 | OPML import / export | feed-engine / http-server |
| R4 | Runtime settings UI (poll interval, TTS voice, translate lang, …) | progress-db / leptos-app / http-server |
| R5 | EPUB bookshelf + upload + reader + reading progress | epub-lib / progress-db / leptos-app / http-server |
| R6 | TTS audio stream + per-word highlight | services / leptos-app |

## Architecture

```
                  ┌──────────────────────────────────┐
                  │  http-server (axum + tower)       │
                  │  ├─ /feeds /articles /opml /settings routes │
                  │  ├─ /epub /translate /tts routes  │
         ───────► │  ├─ Leptos Router (SSR + Hydrate) │ ────┐
   browser        │  └─ background feed poller (tokio)│     │
                  └──┬────────────┬────────────┬─────┘     │
                     │            │            │           │
                     ▼            ▼            ▼           │
               services      progress-db   feed-engine    │
               (Translate/    (sqlx:        (feed-rs +    │
                Tts + Mocks)   feeds +      readability + │
                               articles +   OPML +        │
                               settings)    poller)       │
                                  │            │           │
                                  └────────────┴──── common-text
                                                                │
                  ┌───────────────────────────────────────────┘
                  │  leptos-app (lib + cdylib)
                  │  <FeedList/> <ArticleList/> <ArticleReader/>
                  │  <Bookshelf/> <BookReader/> <SettingsPage/>
                  │  #[server] fns for SSR + Hydration
                  └─────────────────────────────────────────────► WASM
```

## Quick start

```bash
rustup show          # auto-installs 1.92.0 via rust-toolchain.toml
cargo check          # 7 crates compile (~3 min first time)
cargo test --workspace                     # 116 tests pass (4 wasm ignored)
```

## Running

```bash
# 1. Install the Leptos build tool (one-time)
cargo install cargo-leptos

# 2. Copy the config template and edit feed/translate/tts blocks
cp rust-config.example.json rust-config.json
#   → edit "feed.poll_interval_secs" / "translate.target_lang" / "tts.voice"

# 3a. Dev mode — SSR + hot reload on :3000
cargo leptos watch

# 3b. Release mode — single binary on :8083
cargo leptos build --release
./target/release/http-server rust-config.json
```

Once running, open `http://localhost:8083` → **Feeds** → paste an RSS/Atom URL.
The background poller fetches new articles every `feed.poll_interval_secs`
(default 900s). To import existing subscriptions, use **Import OPML** on the
Feeds page.

Data paths (Plan C isolation — never touches Python-side files):

| Asset | Path (from `rust-config.json`) |
|-------|-------------------------------|
| SQLite DB | `rust-data/epub_progress_rust.db` |
| EPUB uploads | `rust-epub-books/` |
| Inject script | `crates/http-server/assets/_inject_rs.js` |

## Configuration

Runtime-overridable settings live in the `settings` SQLite table and are
editable from the **Settings** page. The `rust-config.json` file provides the
initial defaults:

| Key | Default | Purpose |
|-----|---------|---------|
| `feed.poll_interval_secs` | `900` | Background poller interval |
| `feed.fetch_timeout_secs` | `30` | Per-feed HTTP timeout |
| `feed.user_agent` | `miniflux-reader-rs/0.2.0` | User-Agent header |
| `translate.target_lang` | `zh-CN` | Bilingual translation target |
| `tts.voice` | `zh-CN-XiaoxiaoNeural` | Edge-TTS voice |
| `tts.rate` | `+0%` | TTS speaking rate |

## Docker

```bash
docker build -t miniflux-reader-rs:v0.2.0 .
docker run -p 8083:8083 \
  -v $(pwd)/rust-config.json:/app/rust-config.json \
  -v $(pwd)/rust-data:/app/rust-data \
  -v $(pwd)/rust-epub-books:/app/rust-epub-books \
  miniflux-reader-rs:v0.2.0
```

Single-binary image, no external dependencies (no Miniflux, no Python, no
Node.js). Migrations 001–009 run automatically on first boot.

## Contributing — TDD + branch flow (enforced)

| Guardrail | Rule |
|-----------|------|
| Branch strategy | Long-lived `rust` feature branch; every unit of work is `feat/*` or `chore/*` → PR to `rust`. |
| PR to `main` | Reserved for major releases. All feature work goes to `rust`. |
| TDD order | **RED first.** A PR's first commit must contain ONLY new failing tests (implementation not yet added) and CI must show them FAIL. |
| Status checks | 4 non-negotiable green before any PR may merge: (1) `cargo fmt --all -- --check`, (2) `cargo clippy --workspace --all-targets -- -D warnings`, (3) `cargo test --workspace` (NOT `--ignored`), (4) `sqlx migrate run` against empty SQLite, version ≥ 9. |
| External-network tests | Put anything that calls Google Translate / edge-TTS upstream behind `#[ignore]`. CI never runs them. |
| Data isolation | Per Plan C. Never read or write Python-side `config.json`, `data/`, or `epub-books/`. All paths must use `rust-*` prefix. |

## Documents

| File | Location |
|------|----------|
| Design spec (v0.1.0 MVP) | `docs/specs/2026-07-30-rust-leptos-mvp-design.md` |
| v0.2.0 design spec (RSS engine) | `docs/superpowers/specs/2026-07-30-remove-miniflux-rss-engine-design.md` |
| v0.2.0 implementation plan | `docs/superpowers/plans/2026-07-30-remove-miniflux-rss-engine.md` |
| v0.1.0 release notes | `RELEASE_NOTES-v0.1.0.md` |
| v0.2.0 release notes | `RELEASE_NOTES-v0.2.0.md` |
| Python reference project (do not cross-contaminate data) | [miniflux-reader](https://github.com/lizhaoming1/miniflux-reader) |

## License

MIT. See `LICENSE`.
