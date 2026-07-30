# 去除 Miniflux 依赖 + 自建 RSS 引擎设计规格

- **创建日期**：2026-07-30
- **版本目标**：v0.2.0
- **前置版本**：v0.1.0（Rust + Leptos MVP，依赖 Miniflux）
- **迁移策略**：一刀切重写（一次性移除所有 Miniflux 代码，从零构建 RSS 引擎）

---

## 1. 目标与范围

### 1.1 目标

将 miniflux-reader-rs 从 Miniflux 代理前端升级为**独立统一阅读平台**，内置 RSS 订阅引擎，同时保留 EPUB 阅读、双语翻译注入、TTS 朗读能力。不再依赖任何外部 RSS 服务。

### 1.2 核心变更

| 变更 | 说明 |
|------|------|
| 删除 Miniflux 客户端 | 移除 `services/src/miniflux_client.rs`（login + forward） |
| 删除 catch-all 代理层 | 移除 `http-server/src/proxy_layer.rs`（MinifluxProxyLayer） |
| 新增 RSS 引擎 | 新建 `feed-engine` crate：feed 解析、文章抓取、定时轮询 |
| 扩展数据库 | `progress-db` 新增 feeds + articles 表及 repository |
| 重构路由 | 删除 `/mf-login` + proxy fallback，新增 feed/article/opml 路由 |
| 重构前端 | `MinifluxArticle` → `ArticleReader`，新增 FeedList/ArticleList 组件 |
| 无认证单用户 | 不做登录/会话管理，自我托管工具定位 |

### 1.3 保留不变

- `common-text` crate：分句、切块、双语 HTML 渲染 — 完全不动
- `epub-lib` crate：EPUB 解析、上传、safe_name — 完全不动
- `services` 中的 translate/tts：trait + Mock + Reqwest 实现 — 完全不动
- `progress-db` 中现有 ReadingProgress/Book/BookRepository/ProgressRepository — 完全不动

### 1.4 YAGNI — 明确不做

1. 用户认证/登录/会话 — 单用户自我托管，无认证需求
2. 多用户/角色/权限隔离
3. 文章全文搜索 — MVP 只按时间线列表浏览
4. feed 图标/favicon 抓取
5. 翻译/TTS 结果磁盘缓存 — 仅进程内

---

## 2. 架构总览

### 2.1 Crate 结构

```
crates/
├── common-text/     不变（分句/切块/双语HTML）
├── epub-lib/        不变（EPUB解析/上传）
├── progress-db/     扩展：新增 feeds + articles 表 + repository
├── services/        修改：删除 miniflux_client，保留 translate/tts
├── feed-engine/     新增：RSS feed 解析 + 文章抓取 + 定时轮询 + OPML
├── http-server/     修改：删除 proxy_layer，新增 article/feed/opml 路由
└── leptos-app/      修改：MinifluxArticle → ArticleReader，新增 FeedList/ArticleList
```

### 2.2 数据流

```
用户 → http-server(:8083)
         ├─ /feeds             → progress-db (feeds 表)
         ├─ /articles          → progress-db (articles 表)
         ├─ /article/:id       → 正文提取 → 翻译注入 → TTS → Leptos SSR
         ├─ /opml/import       → feed-engine → progress-db
         ├─ /opml/export       → feed-engine ← progress-db
         ├─ /epub/*            → epub-lib → progress-db (不变)
         ├─ /translate         → services (不变)
         ├─ /tts               → services (不变)
         └─ /healthz           → (不变)

后台任务：feed-engine 定时轮询 feed → 解析 → 存入 articles 表
```

与旧架构的核心区别：**不再有 catch-all 代理**，所有路由显式注册，文章内容来自自有 SQLite 数据库而非转发 Miniflux HTML。

---

## 3. feed-engine crate 设计

### 3.1 职责

RSS feed 解析、文章抓取、定时轮询调度、OPML 导入导出、正文提取。纯异步库，无 HTTP 路由，被 `http-server` 调用。

### 3.2 关键类型

```rust
/// 解析结果（fetch_feed 返回）
pub struct ParsedFeed {
    pub title: String,
    pub site_url: String,
    pub articles: Vec<ParsedArticle>,
}

pub struct ParsedArticle {
    pub guid: String,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub summary: String,
    pub content_html: Option<String>,
    pub published_at: DateTime<Utc>,
}
```

### 3.3 核心函数

| 函数 | 签名 | 说明 |
|------|------|------|
| `fetch_feed` | `async (url: &str, timeout: Duration) -> Result<ParsedFeed>` | 用 reqwest 拉 feed，用 feed-rs crate 解析 RSS/Atom |
| `sync_feed` | `async (pool: &SqlitePool, feed_id: i64) -> Result<usize>` | 拉取指定 feed，新文章 upsert 到 articles 表，返回新增数 |
| `sync_all_feeds` | `async (pool: &SqlitePool) -> Result<usize>` | 遍历所有 feed 逐个 sync，单个失败不中断，返回总新增数 |
| `start_poller` | `async (pool: SqlitePool, interval: Duration)` | 启动后台 tokio task，每 interval 调用 sync_all_feeds |
| `extract_content` | `async (url: &str, timeout: Duration) -> Result<String>` | 用 readability crate 从原始 URL 抓取并提取正文 HTML |
| `parse_opml` | `fn (xml: &str) -> Vec<String>` | 解析 OPML XML，提取所有 xmlUrl |
| `export_opml` | `fn (feeds: &[Feed]) -> String` | 生成标准 OPML 2.0 文档 |

### 3.4 正文提取策略

| 场景 | 策略 |
|------|------|
| feed 提供 `<content:encoded>` 全文 | 直接存储，阅读时无需额外抓取 |
| feed 只提供 `<description>` 摘要 | 存储摘要，阅读时按需 extract_content(url) 提取 |

### 3.5 定时轮询

```rust
pub async fn start_poller(pool: SqlitePool, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        if let Err(e) = sync_all_feeds(&pool).await {
            tracing::warn!("feed sync error: {e}");
        }
    }
}
```

- 首次启动立即执行一次全量同步（interval 第一次 tick 即时）
- 单个 feed 失败不中断其他 feed，错误记录到 `feeds.fetch_error`
- `fetch_timeout_secs` 作为 reqwest 请求超时

### 3.6 OPML 格式

```xml
<opml version="2.0">
  <head><title>miniflux-reader-rs subscriptions</title></head>
  <body>
    <outline type="rss" text="Hacker News" xmlUrl="https://news.ycombinator.com/rss"/>
  </body>
</opml>
```

导入：`parse_opml` → 逐 URL `FeedRepository::add` + `sync_feed` → 返回成功/失败计数。
导出：`FeedRepository::list` → `export_opml` → 返回 text/xml。

### 3.7 依赖

- `feed-rs`：RSS/Atom 解析（支持 RSS 2.0 / Atom 1.0 / RSS 1.0）
- `readability`：正文提取
- `reqwest`（已有 workspace 依赖）：抓取 feed
- `tokio`（已有）：异步运行时 + 后台任务
- `quick-xml`（已有）：OPML XML 生成
- `progress-db`：调用 repository 写入 articles

`feed-engine` 调用 `progress-db` 的 repository，自身不直接操作 SQL。

---

## 4. progress-db 扩展设计

### 4.1 新增 Migration

```
migrations/
├── 001-006               已有
├── 007_create_feeds.sql      新增
├── 008_create_articles.sql  新增
└── 009_create_settings.sql  新增
```

### 4.2 007_create_feeds.sql

```sql
CREATE TABLE IF NOT EXISTS feeds (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    url           TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL DEFAULT '',
    site_url      TEXT NOT NULL DEFAULT '',
    last_fetched  TEXT,
    fetch_error   TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 4.3 008_create_articles.sql

```sql
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
```

### 4.4 新增 Repository

```rust
pub struct FeedRepository { pool: SqlitePool }
impl FeedRepository {
    pub async fn add(&self, url: &str) -> Result<Feed>;
    pub async fn remove(&self, id: i64) -> Result<()>;
    pub async fn list(&self) -> Result<Vec<Feed>>;
    pub async fn get(&self, id: i64) -> Result<Feed>;
    pub async fn update_fetched(&self, id: i64, error: Option<&str>) -> Result<()>;
}

pub struct ArticleRepository { pool: SqlitePool }
impl ArticleRepository {
    pub async fn upsert(&self, feed_id: i64, article: &ParsedArticle) -> Result<bool>;
    pub async fn list_unread(&self, feed_id: Option<i64>, limit: u32) -> Result<Vec<Article>>;
    pub async fn list_all(&self, feed_id: Option<i64>, limit: u32, offset: u32) -> Result<Vec<Article>>;
    pub async fn get(&self, id: i64) -> Result<Article>;
    pub async fn set_read(&self, id: i64, read: bool) -> Result<()>;
    pub async fn mark_all_read(&self, feed_id: Option<i64>) -> Result<()>;
    pub async fn count_unread(&self) -> Result<i64>;
}
```

### 4.5 新增模型

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct Feed {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub site_url: String,
    pub last_fetched: Option<String>,
    pub fetch_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, sqlx::FromRow)]
pub struct Article {
    pub id: i64,
    pub feed_id: i64,
    pub guid: String,
    pub title: String,
    pub url: String,
    pub author: Option<String>,
    pub summary: String,
    pub content_html: Option<String>,
    pub published_at: String,
    pub read: bool,
}
```

### 4.6 不变项

- ReadingProgress / ProgressRepository / BookRepository / Book — 完全不动
- run_migrations 函数 — 不动，自动包含新 migration（含 009_create_settings.sql）
- DbError 枚举 — 不动（已有 Sqlx + Migrate + NotFound）

### 4.7 新增 SettingsRepository

```rust
pub struct SettingsRepository { pool: SqlitePool }
impl SettingsRepository {
    pub async fn get_all(&self) -> Result<HashMap<String, String>>;
    pub async fn get(&self, key: &str) -> Result<Option<String>>;
    pub async fn set_many(&self, entries: &HashMap<String, String>) -> Result<()>;
}
```

详见第 13 节后台设置界面。

---

## 5. http-server 路由重构

### 5.1 删除的路由

| 路由 | 删除原因 |
|------|---------|
| `POST /mf-login` | 无认证需求 |
| `.fallback(proxy_fallback)` | 不再代理外部服务 |

### 5.2 新增的路由

| 方法 | 路径 | 处理函数 | 说明 |
|------|------|---------|------|
| GET | `/feeds` | `list_feeds` | 列出所有订阅源（JSON） |
| POST | `/feeds` | `add_feed` | 添加订阅源（body: `{"url":"..."}`） |
| DELETE | `/feeds/:id` | `remove_feed` | 删除订阅源 |
| POST | `/feeds/sync` | `sync_all` | 手动触发全量同步 |
| GET | `/articles` | `list_articles` | 文章列表（query: `?feed_id=&unread_only=&limit=&offset=`） |
| GET | `/articles/:id` | `get_article` | 获取单篇文章详情（含双语注入） |
| POST | `/articles/:id/read` | `set_article_read` | 标记已读/未读（body: `{"read":true}`） |
| POST | `/articles/read-all` | `mark_all_read` | 批量标记已读（query: `?feed_id=`） |
| GET | `/articles/unread-count` | `unread_count` | 未读数量 |
| POST | `/opml/import` | `opml_import` | OPML 导入（text body） |
| GET | `/opml/export` | `opml_export` | OPML 导出（text/xml） |
| GET | `/settings` | `get_settings` | 返回当前所有可改配置（JSON） |
| PUT | `/settings` | `update_settings` | 批量更新配置（body: `{"key":"value",...}`） |

### 5.3 保留不变的路由

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/healthz` | 健康检查 |
| POST | `/epub/upload` | EPUB 上传 |
| GET/POST | `/epub/api/progress/:safe_name` | 进度读写 |
| GET | `/translate` | 翻译 |
| GET | `/tts` | TTS 音频 |
| POST | `/tts_highlight` | TTS 高亮 |

### 5.4 AppState 变更

```rust
// 删除
pub miniflux: Arc<MinifluxClient>

// 新增
pub feed_repo: FeedRepository,
pub article_repo: ArticleRepository,
pub settings_repo: SettingsRepository,
```

`AppState::new` 签名相应调整。`main.rs` 删除 MinifluxClient 构造，新增 repository 构造，并启动 `feed_engine::start_poller` 后台任务。

### 5.5 文章阅读流程

```
GET /articles/:id
  → ArticleRepository::get(id) 取文章
  → 若 content_html 为空 → feed_engine::extract_content(url) 用 readability 抓取正文
  → common_text::split_sentences 分句
  → services::translate_batch 翻译
  → common_text::render_bilingual_div 渲染双语 HTML
  → 返回完整 HTML（内嵌 <script src="/_inject_rs.js">）
```

翻译注入不再在代理层做，而是在 `get_article` handler 中直接完成。`common_text` 的所有分句/翻译/渲染逻辑完整复用。

### 5.6 配置变更

`rust-config.example.json` 删除 `miniflux` 块，新增 `feed` 块：

```json
{
    "listen_addr": "0.0.0.0:8083",
    "feed": {
        "poll_interval_secs": 900,
        "fetch_timeout_secs": 30,
        "user_agent": "miniflux-reader-rs/0.2.0"
    },
    "paths": { ... },
    "translate": { ... },
    "tts": { ... },
    "log_filter": "..."
}
```

`config.rs` 中 `MinifluxCfg` → `FeedCfg`，`AppConfig.miniflux` → `AppConfig.feed`。

---

## 6. leptos-app 前端组件重构

### 6.1 删除/重命名

| 原组件 | 操作 | 说明 |
|--------|------|------|
| `MinifluxArticle` | 重命名为 `ArticleReader` | 双语渲染逻辑不变，仅改名称 |
| `BilingualToggle` | 保留不变 | 纯 UI 组件 |

### 6.2 新增组件

```rust
#[component]
pub fn FeedList(feeds: Vec<FeedInfo>) -> impl IntoView;
// 显示每个 feed 的标题 + 未读数 + 删除按钮

#[component]
pub fn AddFeedForm() -> impl IntoView;
// POST /feeds，单个 URL 输入框

#[component]
pub fn ArticleList(articles: Vec<ArticleSummary>) -> impl IntoView;
// 每条显示标题 + 来源 + 时间 + 已读状态，点击跳转 /article/:id

#[component]
pub fn ArticleReader(sentences: Vec<BilingualSentence>, lang: ToggleLang) -> impl IntoView;
// 从原 MinifluxArticle 重命名，渲染逻辑完全相同

#[component]
pub fn OpmlActions() -> impl IntoView;
// 导入按钮（文件选择 → POST /opml/import）+ 导出链接（GET /opml/export）

#[component]
pub fn SettingsPage() -> impl IntoView;
// 表单展示可改配置项（轮询间隔、超时、UA、翻译语言、TTS 语音）
// 保存按钮 → PUT /settings
```

### 6.3 新增数据模型

```rust
#[derive(Serialize, Deserialize)]
pub struct FeedInfo {
    pub id: i64,
    pub title: String,
    pub site_url: String,
    pub unread_count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: String,
    pub title: String,
    pub url: String,
    pub published_at: String,
    pub read: bool,
}
```

### 6.4 保留不变的组件

- Bookshelf / UploadForm — EPUB 书架
- BookReader / TtsPlayer — EPUB 阅读器 + TTS
- debounce_fire_count — 纯函数
- ToggleLang / BookInfo — 数据模型

### 6.5 App 根组件路由

```rust
<Router>
  <Routes>
    <Route path="/" view=HomePage />               // 文章列表（默认首页）
    <Route path="/feeds" view=FeedsPage />          // 订阅源管理
    <Route path="/article/:id" view=ArticlePage />  // 文章阅读
    <Route path="/settings" view=SettingsPage />     // 运行时设置
    <Route path="/epub" view=BookshelfPage />        // EPUB 书架
    <Route path="/epub/read/:name" view=ReaderPage /> // EPUB 阅读
  </Routes>
</Router>
```

首页从原来的 Miniflux 代理页改为自有文章列表。`/article/:id` 页面内嵌 `ArticleReader` + `TtsPlayer`。

---

## 7. services crate 清理

### 7.1 删除

- `services/src/miniflux_client.rs` — 整个模块删除
- `services/tests/miniflux_client.rs` — 7 个测试整体删除
- `lib.rs` 中 `pub mod miniflux_client` 和 `pub use miniflux_client::{...}` — 删除

### 7.2 保留

- `error.rs`：ServiceError 枚举（TextTooLong / Timeout / Network / Upstream）— 全部保留，translate/tts 的 reqwest 实现仍用 Upstream/Network
- `translate.rs` / `tts.rs` — 完全不动

---

## 8. 错误处理

`AppHttpError` 映射不变：

| 错误 | HTTP 状态码 | code |
|------|------------|------|
| ServiceError::Network | 502 | BAD_GATEWAY |
| ServiceError::Timeout | 504 | GATEWAY_TIMEOUT |
| ServiceError::TextTooLong | 413 | PAYLOAD_TOO_LARGE |
| ServiceError::Upstream | 502 | UPSTREAM |
| DbError::NotFound | 404 | NOT_FOUND |
| DbError (其他) | 500 | DB_ERROR |
| EpubError | 400 | EPUB_ERROR |
| BadRequest | 400 | BAD_REQUEST |
| Internal | 500 | INTERNAL |

`get_article` 中 readability 抓取网络失败复用 `AppHttpError::Service(ServiceError::Network)` → 502。

---

## 9. 测试策略

### 9.1 删除的测试

| 测试文件 | 测试数 | 原因 |
|---------|--------|------|
| `services/tests/miniflux_client.rs` | 7 | 源码删除 |
| `http-server/tests/proxy_wiremock.rs` | 11 | 代理层删除 |
| `http-server/tests/integration_full.rs` T1 (代理双语注入) | 1 | 代理层删除 |
| `http-server/tests/integration_full.rs` T2 (login cookie) | 1 | /mf-login 删除 |
| 合计删除 | 20 | |

### 9.2 修改的测试

| 测试文件 | 变更 |
|---------|------|
| `integration_full.rs` T0 (进度持久化) | 更新 test_state() 签名 |
| `status_codes.rs` T0 (上游 404 透传) | 改写为自有路由 404 |
| `status_codes.rs` T1 (畸形 JSON) | 更新 test_state() 签名 |
| `tts_translate_routes.rs` | 更新 test_state() 签名 |
| `epub_routes.rs` | 更新 test_state() 签名 |
| `config_load.rs` | 断言从 cfg.miniflux.* 改为 cfg.feed.* |
| `ssr_render.rs` T3/T3b | MinifluxArticle → ArticleReader |

### 9.3 新增的测试

| 测试文件 | 测试数 | 内容 |
|---------|--------|------|
| `feed-engine/tests/feed_parse.rs` | ~5 | RSS 2.0 / Atom 1.0 解析（内嵌 XML fixture） |
| `feed-engine/tests/sync_feed.rs` | ~5 | mock HTTP 上游 → sync_feed → 验证 articles 表 |
| `feed-engine/tests/opml.rs` | ~3 | OPML 导入解析 + 导出生成 |
| `progress-db/tests/feeds_table.rs` | ~5 | add/list/remove/update_fetched |
| `progress-db/tests/articles_table.rs` | ~5 | upsert 去重、list_unread、set_read、mark_all_read |
| `http-server/tests/feed_routes.rs` | ~5 | add/list/delete feed，手动 sync |
| `http-server/tests/article_routes.rs` | ~5 | get_article 含双语注入、set_read、unread_count |
| `http-server/tests/opml_routes.rs` | ~3 | 导入/导出 roundtrip |
| `leptos-app/tests/ssr_render.rs` (新增) | ~3 | FeedList/ArticleList 空状态/非空 |
| `leptos-app/tests/component_interaction.rs` (新增) | ~3 | AddFeedForm、OpmlActions |
| `progress-db/tests/settings_table.rs` | ~3 | get_all 空表、set_many 后 get_all 验证、单 key get |
| `http-server/tests/settings_routes.rs` | ~2 | GET 返回默认值、PUT 更新后 GET 验证 |
| 合计新增 | ~51 | |

### 9.4 测试数量估算

- 删除 20，新增 51，净增 31
- 最终约 104 - 20 + 51 = **135 个测试**
- CI migrate 断言更新为 version == 9

---

## 10. CI 更新

| CI 检查 | 变更 |
|---------|------|
| fmt | 无变更 |
| clippy | 无变更（新增 crate 自动纳入 `--workspace`） |
| test | 无变更（`--workspace` 覆盖新 crate） |
| migrate | 断言 version == 9，count == 9 |
| docker | 镜像标签更新为 v0.2.0 |

---

## 11. 版本与发布

- 版本号：v0.1.0 → v0.2.0（minor bump，新增 RSS + 破坏性移除 Miniflux）
- Release Notes 需说明：Miniflux 不再是依赖，v0.1.0 用户需迁移到 v0.2.0 配置格式

---

## 12. 依赖清单

### 12.1 新增 workspace 依赖

| crate | 用于 | 说明 |
|-------|------|------|
| `feed-rs` | feed-engine：RSS/Atom 解析 | latest stable |
| `readability` | feed-engine：正文提取 | latest stable |

### 12.2 移除的依赖

`scraper` 随 `proxy_layer.rs` 删除而移除（旧代理层用它解析 Miniflux HTML 做元素查找）。新设计中文章内容直接来自数据库，双语注入在 handler 中用 `common_text` 处理，不再需要 HTML DOM 解析。

### 12.3 已有不变

reqwest、tokio、sqlx、quick-xml、tracing、thiserror、serde、async-trait — 均已有 workspace 依赖。

---

## 13. 后台设置界面

### 13.1 设计目标

运行时可在 Web 界面修改部分配置，无需重启服务。启动时 JSON 文件提供默认值，settings 表覆盖，运行时以 settings 表为准。

### 13.2 可改配置项

| key | 说明 | 默认来源 |
|-----|------|---------|
| `feed.poll_interval_secs` | 轮询间隔（秒） | JSON feed.poll_interval_secs |
| `feed.fetch_timeout_secs` | 抓取超时（秒） | JSON feed.fetch_timeout_secs |
| `feed.user_agent` | User-Agent | JSON feed.user_agent |
| `translate.target_lang` | 翻译目标语言 | JSON translate.target_lang |
| `tts.voice` | TTS 语音 | JSON tts.voice |

启动固定不可改项：`listen_addr`、`paths.db`、`paths.epub_dir`、`log_filter`。

### 13.3 settings 表

```sql
-- 009_create_settings.sql
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

key-value 存储，简单灵活。

### 13.4 SettingsRepository

```rust
pub struct SettingsRepository { pool: SqlitePool }
impl SettingsRepository {
    pub async fn get_all(&self) -> Result<HashMap<String, String>>;
    pub async fn get(&self, key: &str) -> Result<Option<String>>;
    pub async fn set_many(&self, entries: &HashMap<String, String>) -> Result<()>;
}
```

### 13.5 路由

| 方法 | 路径 | 处理函数 | 说明 |
|------|------|---------|------|
| GET | `/settings` | `get_settings` | 返回当前所有可改配置（JSON） |
| PUT | `/settings` | `update_settings` | 批量更新配置（body: `{"key":"value",...}`） |

### 13.6 运行时生效策略

| 配置项 | 生效方式 |
|--------|---------|
| `feed.poll_interval_secs` | poller 每次循环从 DB 读取，下次循环立即生效 |
| `feed.fetch_timeout_secs` / `user_agent` | 每次 `fetch_feed` 调用前从 DB 读 |
| `translate.target_lang` | 每次 translate 请求从 DB 读 |
| `tts.voice` | 每次 tts 请求从 DB 读 |

### 13.7 SettingsPage 组件

```rust
#[component]
pub fn SettingsPage() -> impl IntoView;
// 表单展示可改配置项（轮询间隔、超时、UA、翻译语言、TTS 语音）
// 保存按钮 → PUT /settings
```

路由新增 `<Route path="/settings" view=SettingsPage /> // 运行时设置`。

### 13.8 AppState 变更

```rust
pub settings_repo: SettingsRepository,  // 新增
```

### 13.9 测试

| 测试文件 | 测试数 | 内容 |
|---------|--------|------|
| `progress-db/tests/settings_table.rs` | ~3 | get_all 空表、set_many 后 get_all 验证、单 key get |
| `http-server/tests/settings_routes.rs` | ~2 | GET 返回默认值、PUT 更新后 GET 验证 |
