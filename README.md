# miniflux-reader-rs 🔩

> Rust rewrite of [lizhaoming1/miniflux-reader](https://github.com/lizhaoming1/miniflux-reader) — Axum 0.8 + Leptos 0.7 SSR + Hydration MVP, single-binary deploy.

## Scope

Only the 6 MVP core links (see §2 of the design spec). Everything else is YAGNI:

| # | Link          | Crates touched                    |
|---|---------------|-----------------------------------|
| L1 | Miniflux login cookie forward        | services / http-server             |
| L2 | Catch-all proxy + CSP strip + inject | http-server (Tower)                |
| L3 | EPUB bookshelf + upload             | epub-lib / leptos-app / http-server|
| L4 | EPUB reader + progress save/get     | leptos-app / progress-db           |
| L5 | Bilingual translation inject        | common-text / services             |
| L6 | TTS stream + per-word highlight     | services / leptos-app              |

## Architecture

```
                  ┌──────────────────────────────────┐
                  │  http-server (axum + tower)       │
                  │  ├─ Static/login/tts routes       │
         ───────► │  ├─ Leptos Router (SSR + Hydrate) │ ────┐
   browser        │  └─ MinifluxProxyLayer (fallback) │     │
                  └──┬────────────┬────────────┬─────┘     │
                     │            │            │           │
                     ▼            ▼            ▼           │
               services      progress-db   epub-lib        │
               (Translate/    (sqlx)        (zip+xml)       │
                Tts + Mocks)      │            │           │
                                  │            │           │
                                  └────────────┴──── common-text
                                                                │
                  ┌───────────────────────────────────────────┘
                  │  leptos-app (lib + cdylib)
                  │  <Bookshelf/> / <BookReader/> / <MinifluxArticle/>
                  │  #[server] SaveProgress / LoadProgress / ToggleLang …
                  └─────────────────────────────────────────────► WASM
```

## Quick start (scaffold — PR#1 only)

```bash
rustup show      # will auto-install 1.81 via rust-toolchain.toml
cargo check      # passes; 6 crates compile (~30s first time)
cargo test       # 1 placeholder test passes (see tests/hello_scaffold.rs)
```

The 7 subsequent PRs then add behaviour one crate at a time (see §6.3 of the
[design spec](docs/specs/2026-07-30-rust-leptos-mvp-design.md) for the exact 9-PR timeline).

## Running (after PR#7 merges)

```bash
cp rust-config.example.json rust-config.json
# edit miniflux credentials
cargo install cargo-leptos
cargo leptos watch         # SSR + hot reload on :3000 (dev)
cargo leptos build --release && ./target/release/http-server  # single binary
```

## Contributing — TDD + branch flow (enforced)

| Guardrail | Rule |
|-----------|------|
| Branch strategy | Long-lived `rust` feature branch; every unit of work is `feat/*` or `chore/*` → PR to `rust`. |
| PR to `main` | Reserved for Scaffold (PR#1) + Final Release (PR#9). All others go to `rust`. |
| TDD order | **RED first.** A PR's first commit must contain ONLY new failing tests (implementation not yet added) and CI must show them FAIL. |
| Status checks | 4 non-negotiable green before any PR may merge: (1) `cargo fmt --all -- --check`, (2) `cargo clippy --workspace --all-targets -- -D warnings`, (3) `cargo test --workspace` (NOT `--ignored`), (4) `sqlx migrate run` against empty SQLite, version ≥ 6. |
| External-network tests | Put anything that calls Google Translate / edge-TTS / Miniflux upstream behind `#[ignore]`. CI never runs them. |
| Data isolation | Per Plan C. Never read or write Python-side `config.json`, `data/`, or `epub-books/`. All paths must use `rust-*` prefix. |

## Documents

| File | Location |
|------|----------|
| Design spec | `docs/specs/2026-07-30-rust-leptos-mvp-design.md` |
| Implementation plan (after spec review) | `docs/plans/2026-07-30-rust-leptos-mvp.md` |
| Python reference project (do not cross-contaminate data) | [miniflux-reader](https://github.com/lizhaoming1/miniflux-reader) |

## License

MIT. See `LICENSE`.
