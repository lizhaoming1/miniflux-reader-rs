//! PR#7 SSR render tests — T0..T3.
//!
//! Verifies that `<Bookshelf/>`, `<BookReader/>`, and `<ArticleReader/>`
//! render the expected HTML via `RenderHtml::to_html()` (SSR mode).
//!
//! Run with: `cargo test -p leptos-app --features ssr --test ssr_render`

#![cfg(feature = "ssr")]

use common_text::BilingualSentence;
use leptos::prelude::*;
use leptos_app::{
    ArticleList, ArticleReader, BookInfo, BookReader, Bookshelf, FeedInfo, FeedList, ToggleLang,
};

/// Helper: wrap a closure in a fresh reactive `Owner` so signals/arenas work.
fn with_owner<T>(f: impl FnOnce() -> T) -> T {
    let owner = Owner::new();
    owner.with(f)
}

// ---------- T0: empty Bookshelf → "暂无 EPUB" ----------

#[test]
fn t0_empty_bookshelf_shows_placeholder() {
    let html = with_owner(|| view! { <Bookshelf books=vec![] /> }.to_html());
    assert!(
        html.contains("暂无 EPUB"),
        "expected placeholder text, got: {html}"
    );
}

// ---------- T1: 2 books → 2 `<article class=book-card>` ----------

#[test]
fn t1_two_books_render_two_cards() {
    let books = vec![
        BookInfo {
            safe_name: "book-one.epub".into(),
            title: "Book One".into(),
            author: "Author A".into(),
        },
        BookInfo {
            safe_name: "book-two.epub".into(),
            title: "Book Two".into(),
            author: "Author B".into(),
        },
    ];
    let html = with_owner(|| view! { <Bookshelf books=books.clone() /> }.to_html());
    // Count only the `<article class="book-card"` opening tags, not the
    // `book-card__title` / `book-card__author` sub-elements.
    let card_count = html.matches("<article class=\"book-card\"").count();
    assert_eq!(
        card_count, 2,
        "expected 2 book-card articles, got {card_count}; html: {html}"
    );
    assert!(
        html.contains("Book One"),
        "missing title Book One; html: {html}"
    );
    assert!(
        html.contains("Book Two"),
        "missing title Book Two; html: {html}"
    );
}

// ---------- T2: BookReader percent=42 → data-percent="42" ----------

#[test]
fn t2_book_reader_renders_data_percent_attr() {
    let html = with_owner(|| {
        view! {
            <BookReader safe_name="test.epub".to_string() percent=42.0 />
        }
        .to_html()
    });
    assert!(
        html.contains("data-percent=\"42\""),
        "expected data-percent=\"42\" attribute, got: {html}"
    );
}

// ---------- T3: ArticleReader ToggleLang Original → no sentence--zh; Bilingual → present ----------

#[test]
fn t3_article_reader_toggle_original_hides_zh() {
    let sentences = vec![BilingualSentence {
        src: "Hello world.".into(),
        zh: "你好世界。".into(),
        src_start: 0,
        src_end: 12,
    }];
    let html = with_owner(|| {
        view! {
            <ArticleReader sentences=sentences.clone() lang=ToggleLang::Original />
        }
        .to_html()
    });
    assert!(
        !html.contains("sentence--zh"),
        "Original mode should NOT render zh, got: {html}"
    );
}

#[test]
fn t3b_article_reader_toggle_bilingual_shows_zh() {
    let sentences = vec![BilingualSentence {
        src: "Hello world.".into(),
        zh: "你好世界。".into(),
        src_start: 0,
        src_end: 12,
    }];
    let html = with_owner(|| {
        view! {
            <ArticleReader sentences=sentences.clone() lang=ToggleLang::Bilingual />
        }
        .to_html()
    });
    assert!(
        html.contains("sentence--zh"),
        "Bilingual mode should render zh, got: {html}"
    );
    assert!(
        html.contains("你好世界"),
        "Bilingual mode should render zh text, got: {html}"
    );
}

// ---------- T4: FeedList empty state renders "No feeds yet" ----------

#[test]
fn t4_feedlist_empty_state() {
    let html = with_owner(|| view! { <FeedList feeds=vec![] /> }.to_html());
    assert!(
        html.contains("No feeds yet"),
        "expected empty-state text, got: {html}"
    );
}

// ---------- T5: FeedList with 2 feeds renders titles + unread_count ----------

#[test]
fn t5_feedlist_two_items() {
    let feeds = vec![
        FeedInfo {
            id: 1,
            title: "HN".into(),
            site_url: "".into(),
            unread_count: 3,
        },
        FeedInfo {
            id: 2,
            title: "LWN".into(),
            site_url: "".into(),
            unread_count: 0,
        },
    ];
    let html = with_owner(|| view! { <FeedList feeds=feeds.clone() /> }.to_html());
    assert!(html.contains("HN"), "missing title HN; html: {html}");
    assert!(html.contains("LWN"), "missing title LWN; html: {html}");
    assert!(
        html.contains(">3<"),
        "missing unread count 3; html: {html}"
    );
}

// ---------- T6: ArticleList empty state renders "No articles" ----------

#[test]
fn t6_articlelist_empty_state() {
    let html = with_owner(|| view! { <ArticleList articles=vec![] /> }.to_html());
    assert!(
        html.contains("No articles"),
        "expected empty-state text, got: {html}"
    );
}
