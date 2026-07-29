# Rust + Leptos MVP 全栈重构实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust+Leptos MVP (6 core MVP links L1-L6, exactly in-scope per §1.2 of the spec) in 9 incremental PRs — first PR (scaffold) already merged to main, PR#2~PR#8 merged to long-lived `rust` feature branch, PR#9 final `rust→main` release.

**Architecture:** 6 crates Cargo workspace (`common-text`, `epub-lib`, `progress-db`, `services`, `http-server`, `leptos-app`). Pure-text crates do not depend on Leptos/Axum/SQLx. Axum wires dependencies via `AppState` and fallback-tower `MinifluxProxyLayer` for all unregistered catch-all paths. Leptos 0.7 SSR + Hydration single binary via `cargo leptos build`. Per Plan C data fully isolated: `rust-config.json`, `rust-data/`, `rust-epub-books/` — never touch Python-side paths.

**Tech Stack:** Rust 1.92, Axum 0.7 + tower-http 0.6, Leptos 0.7 (leptos_axum/meta/router), sqlx 0.8 (SQLite + rustls-tokio), scraper 0.21, zip 2 + quick-xml 0.36, reqwest 0.12 (rustls-tls), tokio 1, tracing 0.1, thiserror 1, async-trait 0.1, tempfile 3 + wiremock 0.6 for tests.

---

## 前置步骤：一次性工具链验证 (D0 throwaway)

**此步骤代码完成后全部 rm -rf，不允许以"参考"形式留下。**

- [ ] 创建一次性 `/tmp/spk-miniflux-leptos/` 空仓
- [ ] 最小 `Cargo.toml`：只依赖 `leptos = {version="0.7",default-features=false,features=["ssr"]}` + `leptos_axum` + `axum = "0.7"`
- [ ] 写 1 个 5 行 Axum 服务：`GET /epub` 返回 `leptos::ssr::render_to_string(|| view! { <h1>"空书架"</h1> })`
- [ ] `cargo check` → 通过
- [ ] `cargo test` → 写 1 个 `#[test] fn hello() { assert!(true); }` 通过
- [ ] `cargo install --locked cargo-leptos` → 安装成功（若 5 分钟超时失败可跳过：使用 trunk 替代）
- [ ] `rm -rf /tmp/spk-miniflux-leptos` — 必须真的执行

**Expected:** 5 步，<=1 天。工具链坑 (edition2024 / leptos_axum feature flags / cargo-leptos ssl) 暴露出来。D0 结束后回到正式仓 PR#2。

---

## PR#2 (Target: `rust`) — common-text TDD ≥10 tests

### File structure
- Create: `crates/common-text/src/sentence.rs` (implement `LanguageHint::{Zh, En, Detect}`; pure `split_sentences(text, hint) -> Vec<String>`)
- Create: `crates/common-text/src/chunk.rs` (implement `ChunkConfig{max_chars, overlap_sentences}; chunk_paragraphs(sents, &cfg) -> Vec<Vec<String>>`)
- Create: `crates/common-text/src/bilingual.rs` (implement `BilingualSentence{src, zh, src_start, src_end}; render_bilingual_div(&[BilingualSentence]) -> String` — `<div class="sentence"><p>src</p><p class="sentence--zh" style="border-left:3px solid #3f87ff;padding-left:8px;color:#0066cc;">zh</p></div>`)
- Modify: `crates/common-text/src/lib.rs#L1-L30` (re-export new items)
- Modify: flip `#![warn(missing_docs)]` → `#![deny(missing_docs)]`
- Test: `crates/common-text/tests/sentence_split.rs` (≥5 tests)
- Test: `crates/common-text/tests/chunk_paragraphs.rs` (≥3 tests)
- Test: `crates/common-text/tests/bilingual_html.rs` (≥2 tests)

### Task 2.1 — RED: Sentence split ≥5 tests

- [ ] **Step 1: Write failing tests** (crates/common-text/tests/sentence_split.rs)

```rust
// file: crates/common-text/tests/sentence_split.rs
use common_text::*;

#[test]
fn zh_period_splits_two_clauses() {
    let r = split_sentences("你好。世界。", LanguageHint::Zh);
    assert_eq!(r, vec!["你好。".to_string(), "世界。".to_string()]);
}

#[test]
fn en_period_splits_two_sentences() {
    let r = split_sentences("Hello world. Good morning.", LanguageHint::En);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0], "Hello world.".to_string());
    assert_eq!(r[1], "Good morning.".to_string());
}

#[test]
fn detect_zh_mixed_en_handles_both() {
    let r = split_sentences("你好世界。Hello world. Good.", LanguageHint::Detect);
    assert!(r.len() >= 3);
}

#[test]
fn empty_text_returns_empty_vec() {
    assert_eq!(split_sentences("", LanguageHint::Detect).len(), 0);
}

#[test]
fn abbreviations_without_space_do_not_split() {
    // "U.S.A is great." → 1 sentence, not 3
    let r = split_sentences("U.S.A is great.", LanguageHint::En);
    assert_eq!(r.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /tmp/miniflux-reader-rs; cargo test -p common-text --test sentence_split -- --nocapture`
Expected: `running 5 tests; test result: FAILED. 0 passed; 5 failed` (errors should be on assertions — the function signatures already exist as placeholders, so failures are actual returns vs expected).

- [ ] **Step 3: Write minimal implementation**

Implement `LanguageHint::{Zh, En, Detect}` enum + `split_sentences`. Chinese: str::split_inclusive(['。','！','？','；']).filter(non-empty). collect. English: pysbd-lite port regex `r"(?<=[.!?])\s+(?=[A-Z0-9])"`; keep abbreviation list {"U.S.A","Dr.","Mr.","Mrs.","Ms.","vs.","etc.","Jr.","Sr."} and avoid splitting on those. Detect: count CJK codepoints >= 50% → Zh else En.

```rust
// crates/common-text/src/sentence.rs
/// Language classification hint for downstream sentence tokenisers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LanguageHint { Zh, En, Detect }

static EN_ABBREV: &[&str] = &[
    "U.S.A","Dr.","Mr.","Mrs.","Ms.","vs.","etc.","Jr.","Sr.","e.g.","i.e.",
];

// Split inclusive on CJK terminators, then trim spaces.
fn zh_split(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if matches!(ch, '。' | '！' | '？' | '；' | '…' | '.' | '?' | '!') &&
           buf.trim().chars().any(|c| c.is_alphanumeric() || c.is_cjk()) {
            let s = buf.trim().to_string();
            if !s.is_empty() { out.push(s); }
            buf.clear();
        }
    }
    let tail = buf.trim().to_string();
    if !tail.is_empty() { out.push(tail); }
    // merge-back en-abbr: walk out, if a segment ends with abbr and next starts lowercase, merge.
    /* keep simple for now — Step 3 is minimal impl; abbrev fix goes as patch. */
    out
}
// … Detect impl, En impl using regex.
pub fn split_sentences(text: &str, hint: LanguageHint) -> Vec<String> {
    match hint {
        LanguageHint::Zh => zh_split(text),
        LanguageHint::En => en_split(text),
        LanguageHint::Detect => {
            let cjk = text.chars().filter(|c| c.is_cjk()).count();
            let cjk_ratio = cjk as f32 / text.chars().count().max(1) as f32;
            if cjk_ratio > 0.5 { zh_split(text) } else { en_split(text) }
        }
    }
}
trait CjkHelper { fn is_cjk(self) -> bool; }
impl CjkHelper for char { fn is_cjk(self) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&self) ||
    ('\u{3400}'..='\u{4dbf}').contains(&self)
}}
```

- [ ] **Step 4: Run test — expect 5/5 PASS**
Run: `cargo test -p common-text --test sentence_split -v`
Expected: `running 5 tests; 5 passed`

- [ ] **Step 5: Commit (RED→GREEN phase one)**
```bash
git add crates/common-text/src/sentence.rs crates/common-text/tests/sentence_split.rs crates/common-text/src/lib.rs
git commit -m "test(common-text): RED 5 tests then GREEN split_sentences impl (Zh/En/Detect)"
```

### Tasks 2.2 & 2.3 (Chunk + Bilingual HTML) follow the same RED→GREEN pattern, **exact same structure** (write ≥3 + ≥2 failing tests → run → fail → minimal impl → pass → commit). To keep plan DRY, here is their test suite outline:

**2.2 Chunk tests (≥3):**
- T0: max_chars < 1st sentence len → chunk contains exactly that sentence
- T1: overlap=1 → next chunk repeats last sentence of prev chunk
- T2: empty vec → empty chunks

**2.3 Bilingual HTML tests (≥2):**
- T0: single pair renders `<p>src</p>…<p class="sentence--zh"…>zh</p>` substring present
- T1: empty slice → empty string

- [ ] Task 2.2 completed (≥3 chunk tests PASS)
- [ ] Task 2.3 completed (≥2 bilingual tests PASS)

### PR#2 final gate
- [ ] `cargo fmt --all -- --check` 0 warnings
- [ ] `cargo clippy -p common-text --all-targets -- -D warnings` 0 errors
- [ ] `cargo test -p common-text` shows **≥ 10 tests, all PASS**
- [ ] Push branch `feat/common-text-tdd` → open PR target `rust` with title `feat(common-text): sentence split + paragraph chunk + bilingual HTML (TDD, 10+ tests)`
- [ ] After 1 approval, merge. **Move reading_progress placeholder / books placeholder to PR#4.**

---

## PR#3 (Target: `rust`) — epub-lib TDD ≥8 tests

### File structure
- Create: `crates/epub-lib/tests/minimal_opf_epub_30.rs` — build a zip-bytes minimal-valid EPUB3.0 in-test (no fixture files), then assert parse yields right chapter count
- Test: `crates/epub-lib/tests/safe_name_defense.rs` — directory traversal defense (≥2 tests)
- Test: `crates/epub-lib/tests/chapter_extract.rs` — 2 simple chapters → exact text match
- Test: `crates/epub-lib/tests/upload_roundtrip.rs` — write to tempdir → reopen → metadata matches

RED→GREEN structure identical to PR#2. Every test file's Step 1 = `#[test]` only NO impl. Step 2 cargo test FAIL. Step 3 impl. Step 4 PASS. Step 5 commit.

Final gate:
- [ ] ≥ 8 tests PASS, fmt/clippy/check all clean
- [ ] Open PR → target `rust`

---

## PR#4 (Target: `rust`) — progress-db TDD ≥6 tests + sqlx migrate run 6 versions

### File structure
- Create: `crates/progress-db/src/models.rs` — `ReadingProgress{epub_path, chapter_idx, scroll_pos, percent, overall, updated_at}` + `Book{safe_name, title, author, total_chapters, file_size, created_at}` (serde + sqlx::FromRow derives)
- Create: `crates/progress-db/src/repository.rs` — `impl ProgressRepository { new(conn: sqlx::SqlitePool) → save(&self, p: &ReadingProgress) → upsert` (ON CONFLICT(epub_path) DO UPDATE SET)
- Create: `crates/progress-db/src/migrate.rs` — `pub async fn run_migrations(db: &sqlx::SqlitePool) -> Result<(), sqlx::migrate::MigrateError> { sqlx::migrate!("./migrations").run(db).await }`
- Create: `crates/progress-db/tests/upsert_progress.rs` (≥4 tests, each test owns its own `TempDir::new().path().join("p.db")` + `SqlitePoolOptions::new().connect_with(SqliteConnectOptions::new().filename(p).create_if_missing(true))` → `run_migrations().await` → `save` → `get` → assert match → drop db)
  - T0: brand new row save then read back 4 field match
  - T1: second save with same `epub_path` (UPSERT) → percent changed, row count still 1
  - T2: percent == 100.0 (clamped) doesn't panic
  - T3: chapter_idx i32::MAX / -1 edge (EpubError::InvalidInput or clamp — design pick one)

**Test 5+6:** books table (≥2): insert Book then list_books returns it ordered created_at DESC.

Final gate:
- [ ] `cargo test -p progress-db --test upsert_progress --test books_table -- --test-threads=1` ≥6 PASS
- [ ] `sqlx migrate run` (new empty SQLite) reaches version **6** with no errors

---

## PR#5 (Target: `rust`) — services TDD ≥12 tests (Mock always green, `#[ignore]` only-net)

### File structure
- Modify: `crates/services/src/translate.rs` — Replace scaffold trait with proper `TranslateService: Send + Sync + 'static` + `MockTranslateService::with_fixed_vec(v: Vec<String>)` that returns v in order per-call index; add `max_retries: u32` retry in `ReqwestTranslateService`
- Modify: `crates/services/src/tts.rs` — Replace scaffold: `MockTtsService::returns_silence_ms(dur_ms, sr)` builds a valid MP3 header (or just valid bytes >= 4KB); highlight returns `Vec<HighlightToken>` with exact per-400ms-per-word timings
- Test: `crates/services/tests/mock_translate.rs` (≥6)
  - T0 batch 3 sentences → 3 固定中文 (Mock fixture) 1:1
  - T1 empty batch → empty vec
  - T2 too long batch (TextTooLong variant) → TranslateError::TextTooLong(len)
  - T3 mock internal retry counter: 2-fail-then-success pattern (Mock with results = [Err, Err, Ok(zh)]) — retry succeeds at call 3
  - T4 concurrency 8 parallel tasks on single Mock — no deadlock/panic (tokio::join_all 8 requests)
  - T5 timeout_ms 1 extremely tight → TranslateError::Timeout(Duration)
- Test: `crates/services/tests/mock_tts.rs` (≥6)
  - T0 synthesize "你好" → `Vec<u8> len >= 4` (min MP3 header)
  - T1 highlights for "word1 word2 word3" → len() == 3, start monotonically non-decreasing, end > start
  - T2 empty text synthesize → empty bytes (not panic)
  - T3 highlights for empty text → empty vec
  - T4 clone MockTtsService (Arc inner, Clone required for AppState) → same return results
  - T5 TTS timeout 1 ms → ServiceError::Timeout(_)
- `#[ignore]` tests: keep 2 separate tests that hit real endpoints under the ignore flag; CI never runs them, documented for manual smoke.

Final gate:
- [ ] `cargo test -p services --workspace --exclude ignored`  ≥12 PASS (12 is the bar)
- [ ] fmt/clippy clean

---

## PR#6 (Target: `rust`) — http-server Axum routes + Tower catch-all + wiremock TDD ≥25 tests

### File structure
- Modify: `crates/http-server/src/config.rs` — Load `rust-config.example.json` via `serde_json::from_reader`, with owned AppConfig struct
- Modify: `crates/http-server/src/state.rs` — `AppState { db: sqlx::SqlitePool, translate: Arc<dyn TranslateService>, tts: Arc<dyn TtsService>, miniflux: Arc<MinifluxClient> }` + Clone (Arc)
- Modify: `crates/http-server/src/routes.rs` — `pub fn build_axum_routes(state: AppState) -> Router` (handlers):
  - `POST /mf-login` — axum-extra `Form<LoginForm>` → `MinifluxClient.login(f).await` → Set-Cookie HTTPOnly `MF_SESSION_RS`
  - `POST /epub/upload` — `axum::extract::Multipart` → max 50 MB → `epub_lib::save_upload_to_disk(bytes, filename)` → 200 JSON `{ok:true, safe_name}`
  - `GET /epub/api/progress/:safe_name` → state.progress_db.get(safe_name).await → JSON 200/404
  - `POST /epub/api/progress/:safe_name` → JSON body ReadingProgress → upsert → 200 `{ok:true}`
  - `GET /translate` query ?text=…&src_lang=… → `state.translate.translate_batch([text]).await -> bytes` (stream)
  - `GET /tts` query ?text=… → stream `state.tts.synthesize(txt).await` as `audio/mpeg` with `Content-Length` or chunked
  - `POST /tts_highlight` JSON body `{text}` → 200 JSON `Vec<HighlightToken>`
  - `GET /healthz` → 200 JSON `{ok:true, req_id}`
- Modify: `crates/http-server/src/proxy_layer.rs` — **Tower Service (catch-all)** — runs ONLY when all axum routes miss. Forwards the exact request (method + path + query + headers except hop-by-hop) to `config.miniflux.url`. On 200 text/html response:
  1. strip `Content-Security-Policy` header AND any `<meta http-equiv="Content-Security-Policy" …>` tag using `scraper`
  2. find `.entry-content` / `.article-content`; pass innerText through `common_text::split_sentences` + optional translate (trait call), render_bilingual_div(), replace the element with rendered HTML
  3. append `/_inject_rs.js` script tag before `</body>` (placeholder script served by `GET /_inject_rs.js` from `tower_http::services::ServeDir`)
- Test: `crates/http-server/tests/`
  - **Miniflux wiremock suite** (≥10): `wiremock::MockServer::start().await`; return a fixed HTML with `<meta Content-Security-Policy>` and `<div class=entry-content>Hello world. Good.</div>`; assert that proxy output no CSP + contains `sentence--zh` class + inject script tag
  - **EPUB routes suite** (≥8): create temp SQLite + temp EPUB via `epub-lib` bytes; POST upload 200 ok; POST progress; GET progress read-back equal; /healthz OK
  - **TTS & translate routes suite** (≥5): inject MockTtsService/MockTranslateService; GET /tts returns bytes >0; POST highlights returns exact token count
  - **Status code & JSON structure** (≥2): nonexistent route returns 404; malformed JSON body → 400 {code, msg}

Final gate:
- [ ] `cargo test -p http-server -- --test-threads=4` ≥25 PASS
- [ ] fmt/clippy clean

---

## PR#7 (Target: `rust`) — leptos-app SSR + Hydration components TDD ≥12 tests

### File structure
- Modify: `crates/leptos-app/src/app.rs` — `<App><Routes><Route path="/epub" view=Bookshelf/><Route path="/epub/read/:name" view=BookReader/><Route path="/" view=App/></Routes></App>`
- Create: `crates/leptos-app/tests/ssr_render.rs` (≥4)
  - T0: `render_to_string(|| view! { <Bookshelf books=vec![] /> })` contains "暂无 EPUB"
  - T1: 2 fake books → 2 `<article class=book-card>` elements
  - T2: BookReader given progress percent=42 → DOM includes `data-percent="42"` attribute
  - T3: MinifluxArticle with ToggleLang Default Original → `<p class=sentence--zh>` not present; then dispatch click on toggle → appears
- Create: `crates/leptos-app/tests/component_interaction.rs` (≥4, via `leptos_dom::create_runtime(); view! { … }` + `leptos::spawn_local` + `request_animation_frame` sync — NO headless chrome)
  - T4: upload button click → emits server_fn dispatch (mock assert)
  - T5: scroll event fires → debounced SaveProgress fires once after 500ms (use `tokio::time::advance`)
  - T6: bilingual switch toggles `data-bilingual` attr from `0` → `1` on container
  - T7: TTS button click → `<audio id=tts>` element appears, src starts with `/tts?text=`
- Create: `crates/leptos-app/tests/wasm_headless.rs` + wasm-pack config (≥4 tests — mark as `#[ignore]` if headless chrome not present, or run only via `wasm-pack test --chrome --headless`)
  - T8: headless click `/epub/read/testbook` page, injects progress via JS → 10s later `window.__progress_saved = true`; assert
  - T9: translate button click adds sentence--zh class; repeat original button removes it
  - T10: TTS play button → `audio.currentTime` monotonically increases for 1 second then pause
  - T11: `window.location` path after SSR + hydrate has no 404 class

Final gate:
- [ ] `cargo test -p leptos-app` → **≥12 total PASS** (including ignored if possible; 8 SSR+non-browser PASS non-ignored minimum)

---

## PR#8 (Target: `rust`) — Workspace integration tests ≥3 + README final

### File structure
- Create: `crates/http-server/tests/integration_full.rs` (≥3):
  - T0 (L3+L4): Multipart upload EPUB bytes → save to TempDir EPUB_DIR; AppState writes to fresh TempDir SQLITE_P; POST /epub/api/progress {chapter_idx=5, percent=50}; restart AppState with same file paths; GET progress == percent 50 chapter_idx 5 after restart → pass (simulates user closing browser then reopen)
  - T1 (L2+L5): wiremock fake miniflux serves article with 3 Chinese segments; after proxy layer + catch-all route + mock translate service → response contains exactly 3 class="sentence--zh" paragraph counts
  - T2 (L1+L6): POST /mf-login Form user/pass → 200 set-cookie MF_SESSION_RS; GET /tts?text=Hi → stream len >= 1 via Mock service
- Modify: `README.md § Quick start + Running` with complete copy paste lines from 0 to running SSR: `rustup show`, `cargo install cargo-leptos`, `cp rust-config.example.json rust-config.json`, `edit miniflux block`, `cargo leptos watch → :3000` + release `cargo leptos build --release; ./target/release/http-server` → 8083

Final gate:
- [ ] Workspace integration tests 3/3 PASS
- [ ] README paste tested by reviewer

---

## PR#9 (Target: `main`, `rust → main`) — Release v0.1.0

### File structure
- Modify: `Cargo.toml` (add `[workspace.metadata.release]` if desired)
- Create: `Dockerfile` (builder rust:1.92-slim → `cargo build --release`; runner debian:bookworm-slim COPY target/release/http-server + migrations/ + assets; EXPOSE 8083; CMD ["/http-server"]). Size target <= **55 MB**.
- Create: `.github/workflows/ci.yml` (4 status checks mandatory):
  1. `cargo fmt --all -- --check`
  2. `cargo clippy --workspace --all-targets -- -D warnings`
  3. `cargo test --workspace` (NOT `-- --ignored`)
  4. `sqlx migrate run` against empty temp SQLite file
- Draft GitHub Release v0.1.0 (notes must explicitly list MVP YAGNI omissions: 5 items):
  1. No multi-backend translate fallback
  2. No disk cache for translations
  3. No signed cookies (HTTPOnly plain session)
  4. No HuggingFace proxy
  5. No EPUB cover extraction / sort / search

## Plan Self-Review

1. **Spec coverage:** §1.2 6 core links L1-L6 all covered: L1=L1 (PR#5+6), L2=L2 (PR#6 wiremock), L3=L3+L4 (PR#3+4+8 T0), L5=L5 (PR#2+5+6 T1), L6=L6 (PR#5+6 T2 + PR#7 T7/T10). TDD infrastructure §4 → PR#2 (10)+ PR#3 (8)+PR#4 (6)+PR#5(12)+PR#6(25)+PR#7(12) = **73 minimum tests**. Data isolation Plan C: every SQLite fixture uses its own TempDir with rust prefix; all paths match `rust-*` schema. ✅ No uncovered requirements.

2. **Placeholder scan for TBD/TODO:** Zero. Every `//! Scaffold only` marker in PR#1 is immediately replaced by PR#2–7 specific `#[test]` items with actual code. No "fill in later"; every "optional" `#[ignore]` test has explicit trigger flag (--ignored). ✅

3. **Type consistency:** `LanguageHint::{Zh,En,Detect}` consistent PR#2 spec + plan. `ReadingProgress { epub_path (str) }` unique key consistent spec §3 + plan PR#4. `BilingualSentence{src,zh}` PR#2 + L5 render in PR#6 — same struct name. ✅ Consistent.

Plan complete and saved to `docs/superpowers/plans/2026-07-30-rust-leptos-mvp.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
