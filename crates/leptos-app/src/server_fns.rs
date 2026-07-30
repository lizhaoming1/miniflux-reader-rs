//! `#[server]` functions + SSR fixture helpers.
//!
//! P0: For initial UI wiring, this module exposes plain helper functions
//! that return deterministic fixture data, allowing page components to
//! render real content before the full data-layer integration (P1) is
//! in place. P1 will replace the fixtures with real `#[server]` calls
//! backed by the HTTP API or direct repository access.

use common_text::BilingualSentence;

use crate::{ArticleSummary, BookInfo, FeedInfo, ToggleLang};

/// Build a fixed vector of article summaries for the home-page list.
///
/// Covers the display scenarios that matter for the P0 chain: a mix of
/// read and unread rows, different feeds, and realistic timestamps so
/// the `<ArticleList/>` component renders in all its states.
pub fn fixture_articles() -> Vec<ArticleSummary> {
    vec![
        ArticleSummary {
            id: 1,
            feed_id: 1,
            feed_title: "Hacker News".to_string(),
            title: "Rust 1.92 发布：更紧凑的 DWARF 调试信息与 async 改进"
                .to_string(),
            url: "https://blog.rust-lang.org/2026/07/25/Rust-1.92.html".to_string(),
            published_at: "2026-07-28T09:15:00Z".to_string(),
            read: false,
        },
        ArticleSummary {
            id: 2,
            feed_id: 2,
            feed_title: "Lobsters".to_string(),
            title: "用 Leptos 构建 SSR 优先的 Web 应用：实战经验"
                .to_string(),
            url: "https://example.com/leptos-ssr-notes".to_string(),
            published_at: "2026-07-27T14:42:00Z".to_string(),
            read: false,
        },
        ArticleSummary {
            id: 3,
            feed_id: 1,
            feed_title: "Hacker News".to_string(),
            title: "SQLite 3.48.0 新增内置向量搜索功能".to_string(),
            url: "https://sqlite.org/releaselog/3_48_0.html".to_string(),
            published_at: "2026-07-26T20:05:00Z".to_string(),
            read: true,
        },
        ArticleSummary {
            id: 4,
            feed_id: 3,
            feed_title: "Planet Rust".to_string(),
            title: "从零构建一个 feed-rs 爬虫：RSS/Atom 解析实战".to_string(),
            url: "https://blog.example.dev/feed-rs-crawler".to_string(),
            published_at: "2026-07-25T11:30:00Z".to_string(),
            read: true,
        },
        ArticleSummary {
            id: 5,
            feed_id: 2,
            feed_title: "Lobsters".to_string(),
            title: "为什么我们把 Python FastAPI 服务整体迁移到了 Rust Axum"
                .to_string(),
            url: "https://case-study.example/axum-migration".to_string(),
            published_at: "2026-07-24T07:58:00Z".to_string(),
            read: false,
        },
    ]
}

/// Build feed-card fixture rows with unread counts populated so that
/// the `<FeedList/>` component renders meaningful badges.
pub fn fixture_feeds() -> Vec<FeedInfo> {
    vec![
        FeedInfo {
            id: 1,
            title: "Hacker News".to_string(),
            site_url: "https://news.ycombinator.com".to_string(),
            unread_count: 42,
        },
        FeedInfo {
            id: 2,
            title: "Lobsters".to_string(),
            site_url: "https://lobste.rs".to_string(),
            unread_count: 7,
        },
        FeedInfo {
            id: 3,
            title: "Planet Rust".to_string(),
            site_url: "https://planet.rust-lang.org".to_string(),
            unread_count: 0,
        },
    ]
}

/// Build bookshelf fixture entries so `<Bookshelf/>` renders the grid
/// card layout and the "阅读" link points at a plausible reader path.
pub fn fixture_books() -> Vec<BookInfo> {
    vec![
        BookInfo {
            safe_name: "rust-for-rustaceans.epub".to_string(),
            title: "Rust for Rustaceans".to_string(),
            author: "Jon Gjengset".to_string(),
        },
        BookInfo {
            safe_name: "the-rustonomicon.epub".to_string(),
            title: "The Rustonomicon".to_string(),
            author: "The Rust Project".to_string(),
        },
        BookInfo {
            safe_name: "zero-to-production.epub".to_string(),
            title: "Zero To Production In Rust".to_string(),
            author: "Luca Palmieri".to_string(),
        },
    ]
}

/// Fixture article content (bilingual sentence pairs) for the
/// article-reader page. Mirrors the shape that the translation pipeline
/// will emit at runtime; `zh` fields are intentionally populated here
/// so the bilingual toggle renders visibly without any network call.
pub fn fixture_bilingual_sentences() -> (Vec<BilingualSentence>, ToggleLang) {
    let sentences = vec![
        BilingualSentence {
            src: "Rust 1.92 引入了多项关键改进。".to_string(),
            zh: "Rust 1.92 introduces several key improvements.".to_string(),
            src_start: 0,
            src_end: 24,
        },
        BilingualSentence {
            src: "编译器生成的 DWARF 调试信息现在更加紧凑，\
                  二进制体积平均减小 8%。"
                .to_string(),
            zh: "DWARF debug info emitted by the compiler is now more \
                 compact, reducing binaries by 8% on average."
                .to_string(),
            src_start: 24,
            src_end: 80,
        },
        BilingualSentence {
            src: "此外，异步运行时的调度器优化可以使高并发场景下的 \
                  P99 延迟降低约 15%。"
                .to_string(),
            zh: "In addition, scheduler optimisations in the async runtime \
                 reduce P99 latency by approximately 15% under high concurrency."
                .to_string(),
            src_start: 80,
            src_end: 140,
        },
        BilingualSentence {
            src: "更多细节请参考官方发布公告。".to_string(),
            zh: "See the official release notes for further details.".to_string(),
            src_start: 140,
            src_end: 160,
        },
    ];
    (sentences, ToggleLang::Bilingual)
}

/// Return a fixed reading-progress percent for a given EPUB safe-name
/// fixture. Returns 0.0 if the name is not a known fixture.
pub fn fixture_book_progress(safe_name: &str) -> f64 {
    match safe_name {
        "rust-for-rustaceans.epub" => 42.5,
        "the-rustonomicon.epub" => 18.0,
        "zero-to-production.epub" => 0.0,
        _ => 0.0,
    }
}
