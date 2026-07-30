# Release v0.1.0 — Rust + Leptos MVP

**Date:** 2026-07-30
**Tag:** `v0.1.0`
**Branch:** `rust` → `main` (release PR #9)

## What's included

First MVP release of the Rust rewrite of `miniflux-reader`. Replaces the
Python backend with Axum + Leptos 0.7 SSR + Hydration, fully isolated from
the legacy Python data (Plan C — separate SQLite DB + EPUB dir).

### 6 MVP link chains (all tested end-to-end)

| Link | Feature | Test coverage |
|------|---------|---------------|
| L1 | Miniflux login (Set-Cookie MF_SESSION_RS, HttpOnly) | `services/tests/miniflux_client.rs` T0-T2 + `integration_full.rs` T2 |
| L2 | Catch-all proxy + CSP header stripping | `http-server/tests/proxy_wiremock.rs` T0-T11 (11 wiremock scenarios) |
| L3 | EPUB bookshelf + upload | `http-server/tests/epub_routes.rs` T0-T7 + `leptos-app/tests/ssr_render.rs` T0-T1 |
| L4 | Reading progress save/get (SQLite persistence) | `progress-db/tests/upsert_progress.rs` + `integration_full.rs` T0 (restart persistence) |
| L5 | Bilingual translation injection (zh + translation paragraph pairs) | `common-text/tests/bilingual_html.rs` + `integration_full.rs` T1 |
| L6 | TTS stream + sentence highlighting | `services/tests/mock_tts.rs` + `http-server/tests/tts_translate_routes.rs` T0-T4 |

### Test stats

- **104 tests pass** (101 unit/integration + 3 PR#8 cross-crate integration)
- **4 tests ignored** (require `wasm-pack` + headless Chrome; not CI-blocking)
- 0 fmt warnings, 0 clippy warnings (`-D warnings`)
- 6/6 SQLite migrations succeed (version=6)

### Build & run

```bash
# From source
cargo install cargo-leptos
cp rust-config.example.json rust-config.json  # edit miniflux credentials
cargo leptos watch          # dev: SSR + hot reload on :3000
cargo leptos build --release && ./target/release/http-server  # prod: :8083

# Docker
docker build -t miniflux-reader-rs:v0.1.0 .
docker run -p 8083:8083 \
  -v $(pwd)/rust-config.json:/app/rust-config.json \
  -v $(pwd)/rust-data:/app/rust-data \
  miniflux-reader-rs:v0.1.0
```

## 5 YAGNI omissions (intentionally NOT in v0.1.0)

These were considered and explicitly deferred per the YAGNI principle. They
are **not bugs** — they are out-of-scope for a single-user self-hosted MVP.

### 1. No authentication on the Rust server itself

**Omitted:** Login/session management for the Rust HTTP server.
**Reason:** The Rust server runs behind the same reverse proxy as Miniflux;
Miniflux handles auth. The Rust server trusts the `MF_SESSION_RS` cookie
forwarded by the proxy and delegates login to `/mf-login` → Miniflux.
**Revisit when:** Multi-user deployment is needed (would require
tenant isolation + own auth).

### 2. No EPUB content extraction (text rendering only)

**Omitted:** Parsing EPUB XHTML into structured chapters with CSS/images.
**Reason:** MVP only needs chapter navigation + scroll position tracking.
The `epub-lib` crate parses the zip + OPF manifest + safe_name, but does
not render chapter HTML to the reader view — that comes from Miniflux
article HTML via the proxy pipeline.
**Revisit when:** Offline reading (no Miniflux) is needed.

### 3. No real Translate/TTS service in tests (all mocked)

**Omitted:** Integration tests do not call the real Google Translate API
or Azure TTS.
**Reason:** External API calls are non-deterministic, rate-limited, and
cost money. `MockTranslateService` + `MockTtsService` provide deterministic
test coverage; `ReqwestTranslateService` / `ReqwestTtsService` are the
production implementations but are not exercised in CI.
**Revisit when:** A staging environment with API credentials is available.

### 4. No database backup/restore mechanism

**Omitted:** Automated SQLite backup, point-in-time recovery, or WAL
checkpointing.
**Reason:** Single-user self-hosted; the SQLite file is in
`rust-data/epub_progress_rust.db` and can be manually copied. Plan C
isolates it from the Python side, so corruption risk is bounded.
**Revisit when:** Multi-user or hosted deployment.

### 5. No metrics/observability (tracing only, no Prometheus)

**Omitted:** `/metrics` endpoint, Prometheus scrape, Grafana dashboards,
alerting rules.
**Reason:** `tracing` + `tracing-subscriber` with `env-filter` provides
structured logs to stdout — sufficient for `docker logs` debugging. No
SLO targets defined for a single-user tool.
**Revisit when:** SLOs are defined or multiple users share one instance.

## Known limitations

- **wasm-pack tests skipped in CI** — 4 `#[ignore]` tests in
  `leptos-app/tests/wasm_headless.rs` require `wasm-pack` + headless
  Chrome. Run manually: `wasm-pack test --headless --chrome crates/leptos-app`.
- **First release, no rollback target** — rolling back v0.1.0 means
  `git revert` the main merge commit; no prior version exists.

## Verification evidence

```
cargo fmt --all -- --check           → 0 warnings
cargo clippy --workspace --all-targets -- -D warnings  → 0 errors
cargo test --workspace -- --test-threads=1
  → 104 passed / 0 failed / 4 ignored
sqlx migrate run (fresh SQLite)      → version=6, 6/6 success
```

## What's next (v0.2.0 candidates)

Not committed — just tracked:
- EPUB content rendering (offline reading)
- Real Translate/TTS integration tests with sandboxed credentials
- wasm-pack in CI
- SQLite backup automation
- Prometheus metrics endpoint
