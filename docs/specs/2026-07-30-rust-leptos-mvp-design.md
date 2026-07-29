# Rust + Leptos MVP 全栈重构设计规格 (miniflux-reader-rs)

- **创建日期**：2026-07-30
- **所属仓库对**：
  - Python 参考实现：`lizhaoming1/miniflux-reader`
  - Rust 新仓库：`lizhaoming1/miniflux-reader-rs` (本仓，双仓完全隔离)
- **相关文档**：
  - Python 侧设计对照 (不迁移的功能/边界)：https://github.com/lizhaoming1/miniflux-reader/tree/main/docs/superpowers/specs
  - **实施计划**：`docs/plans/2026-07-30-rust-leptos-mvp.md` (brainstorming 通过后由 writing-plans 生成)

---

## 1. 目标与范围

### 1.1 目标
把当前基于 Python (FastAPI + ebooklib + edge-tts + inject.js) 实现的阅读平台，用 Rust 全栈重写为一个单二进制可部署的版本：Axum 做 HTTP 路由 + 代理中间件，Leptos 0.7 SSR+Hydration 做书架/阅读页 UI。

### 1.2 MVP 6 条核心链路
| # | 链路 | 对应 Python 模块 |
|---|------|------------------|
| L1 | Miniflux 登录 + Session Cookie 转发 | `proxy/routes/miniflux.py` + `proxy/services/auth.py` |
| L2 | Miniflux catch-all 代理 (CSP 剥离 + inject_rs.js 注入) | `proxy/routes/miniflux.py` fetch_page |
| L3 | EPUB 书架 + multipart 上传 + 列表页 SSR + Hydration | `epub/templates.py` BOOKSHELF_HTML + `proxy/routes/epub.py` /upload |
| L4 | EPUB 阅读 + 进度 save/get (UPSERT on epub_path unique) | `epub/db.py` + `proxy/routes/epub.py` /progress |
| L5 | 通用双语翻译注入 (Miniflux article + EPUB 阅读页共用) | `common/translate.py` translate_batch |
| L6 | TTS 音频流 + 逐词高亮 JSON | `common/audio.py` + `tts/server.py` |

### 1.3 YAGNI — MVP 明确不做
1. `proxy/services/curl.py` 多后端 fallback — 单后端，失败抛 502
2. `proxy/services/cache.py` 磁盘翻译缓存 — 仅进程内内存缓存
3. `proxy/services/auth.py` Cookie 签名 — 仅 HTTPOnly plain 存 Session
4. `hf-proxy.py` HuggingFace 代理
5. EPUB 封面提取 + 书架搜索 + 排序

### 1.4 数据策略：方案 C 完全隔离
| 资产 | Python 旧仓 | Rust 新仓 (本仓) | 是否迁移？|
|------|------------|----------------|----------|
| 配置 | `config.json` | `rust-config.json` | 否 |
| DB   | `data/epub_progress.db` | `rust-data/epub_progress_rust.db` | 否；独立 CLI 工具再迁 |
| EPUB | `epub-books/*.epub` | `rust-epub-books/<safe_name>.epub` | 否，MVP 重新上传 |

---

## 2. 架构 & 技术栈

### 2.1 Cargo Workspace (6 crates)
```
crates/
├── common-text/   — 纯函数：分句/切块/双语HTML
├── epub-lib/      — EPUB zip+xml 解析 + 上传落盘
├── progress-db/   — sqlx(SQLite, runtime-tokio-rustls) save/get
├── services/      — TranslateService / TtsService (Mock + Reqwest)
├── http-server/   — Axum 主二进制 + Router + Tower catch-all MinifluxProxy
└── leptos-app/    — Leptos 视图 + #[server] fns (SSR + CSR + Hydration)
```

### 2.2 关键依赖 (方案 A crate 栈)
见根 `Cargo.toml` [workspace.dependencies]。全栈 rustls，**无 OpenSSL 系统依赖**。

### 2.3 渲染部署模式
Leptos SSR + Axum + Hydration (单二进制)。路由 3 阶段：① Axum 原生 (login/tts/translate/upload) ② Leptos Router (命名页面) ③ Tower catch-all → MinifluxProxyLayer。

---

## 3. SQLite Schema (progress-db)

见 `/migrations/001_init_progress.sql` + `/migrations/002_init_books.sql`；`/migrations/003…006` 为 noop 占位。

---

## 4. TDD 测试基础设施 (≥ 73 Rust tests)

| Crate | 测试数 ≥ | 方法 |
|-------|----------|------|
| common-text | 10 | pure unit |
| epub-lib    |  8 | include_bytes! fixture EPUB + TempDir |
| progress-db |  6 | **per-test independent TempDir SQLite + migrate** |
| services    | 12 | 90% Mock (default) + 10% #[ignore] only-manual |
| http-server | 25 | Router::oneshot + wiremock::MockServer fake Miniflux |
| leptos-app  | 12 | 4 SSR DOM, 4 leptos_dom_test interaction, 4 wasm-pack headless chrome |

---

## 5. 错误分层 & 可观察性

- 业务错误：`thiserror` enum (TranslateError / TtsError / DbError / EpubError / ProxyError) 每个 → `axum::response::IntoResponse` → HTTP 码 + JSON `{code, msg, req_id}`。
- 强制 tracing 字段：`req_id` (uuid v4 simple) 所有 span；`tower-http::trace::TraceLayer` 根 span 打 `method, path, status, latency_ms`。
- 默认 `RUST_LOG=info,common_text=debug,progress_db=debug,http_server=debug,leptos_app=debug`。

---

## 6. 分支、PR & Status Check (强制执行)

见 `README.md § Contributing — TDD + branch flow`。

**9 PR 时间线：**
| PR# | Target | Content |
|-----|--------|---------|
| 1 | main | Scaffold (本 PR) |
| 2 | rust | common-text TDD |
| 3 | rust | epub-lib TDD |
| 4 | rust | progress-db TDD |
| 5 | rust | services TDD |
| 6 | rust | http-server (axum + Tower layer) TDD |
| 7 | rust | leptos-app views + server fns TDD |
| 8 | rust | workspace integration tests + readme final run guide |
| 9 | main | rust→main release PR (opt + Dockerfile + release notes) |

---

## 7. Spec Self-Review

| 项 | 状态 |
|----|------|
| Placeholder 扫描 | ✅ 003~006 均为 noop，明确写出未来加什么 |
| 字段命名一致 | ✅ epub_path/safe_name 全仓统一；percent 类型 REAL |
| Spec ↔ 6 MVP 链路 覆盖 | ✅ L1-L6 每链路对应 1+ PR + 测试数 |
| 方案 C 数据隔离 落到每一处 | ✅ 路径、schema、README 都带 rust- 前缀 |
