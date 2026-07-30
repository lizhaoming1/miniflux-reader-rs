//! `<App/>` — root component with leptos_router.
//!
//! P0 wiring: every route renders a concrete page component populated
//! with deterministic fixture data. This moves the UI from "all static
//! placeholders" to "the basic navigation chain works" without needing
//! the full data-layer + server-fns infrastructure (that comes in P1).
//!
//! Route conflict note: explicit JSON API routes live at `/feeds` and
//! `/settings` (plural/same names). The Leptos page routes use the
//! `-ui` suffix for those two paths to avoid clashing with the Axum
//! catch-all dispatch order. P1 will move the API under `/api/*` and
//! restore canonical page URLs.

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes, A},
    hooks::use_params_map,
    path,
};

use crate::server_fns::{
    fixture_articles, fixture_bilingual_sentences, fixture_book_progress,
    fixture_books, fixture_feeds,
};
use crate::{
    AddFeedForm, ArticleList, BilingualToggle, BookInfo, BookReader, Bookshelf,
    FeedList, OpmlActions, SettingsPage, UploadForm,
};

/// Top-level page: chronological article list (home page).
#[component]
fn HomePage() -> impl IntoView {
    let articles = fixture_articles();
    view! {
        <section class="page page--home">
            <header class="page__header">
                <h2>"Latest articles"</h2>
                <p class="page__hint">"Browse recent items from all feeds."</p>
            </header>
            <ArticleList articles=articles />
        </section>
    }
}

/// Feed-management page: feed cards + add-form + OPML import/export.
#[component]
fn FeedsPage() -> impl IntoView {
    let feeds = fixture_feeds();
    view! {
        <section class="page page--feeds">
            <header class="page__header">
                <h2>"Subscriptions"</h2>
                <p class="page__hint">"Add feed URLs or import an OPML file to get started."</p>
            </header>
            <FeedList feeds=feeds />
            <AddFeedForm />
            <OpmlActions />
        </section>
    }
}

/// Article reader page. In P0 every id renders the same bilingual
/// fixture body so the toggle + sentence paragraph layout is
/// exercised end-to-end.
#[component]
fn ArticlePage() -> impl IntoView {
    let params = use_params_map();
    let _id = move || {
        params
            .with(|m| m.get("id"))
            .unwrap_or_default()
    };
    let (sentences, lang) = fixture_bilingual_sentences();
    let lang_signal = RwSignal::new(lang);
    view! {
        <section class="page page--article">
            <header class="page__header">
                <a class="back-link" href="/">"← Back to articles"</a>
                <h2>"Rust 1.92 发布说明"</h2>
                <div class="article-toolbar">
                    <button
                        type="button"
                        on:click=move |_| {
                            let cur = lang_signal.get();
                            let next = match cur {
                                crate::ToggleLang::Original => crate::ToggleLang::Bilingual,
                                crate::ToggleLang::Bilingual => crate::ToggleLang::Original,
                            };
                            lang_signal.set(next);
                        }
                    >
                        {move || match lang_signal.get() {
                            crate::ToggleLang::Original => "Show bilingual",
                            crate::ToggleLang::Bilingual => "Original only",
                        }}
                    </button>
                </div>
            </header>
            <BilingualToggle lang=lang_signal.read_only() sentences=sentences />
        </section>
    }
}

/// Runtime settings page: the canonical form lives in
/// `SettingsPage`; P0 uses the static default inputs that the
/// component already embeds.
#[component]
fn SettingsPageWrapper() -> impl IntoView {
    view! {
        <section class="page page--settings">
            <header class="page__header">
                <h2>"Runtime settings"</h2>
                <p class="page__hint">"Adjust feed polling and service options without restarting the server."</p>
            </header>
            <SettingsPage />
        </section>
    }
}

/// EPUB bookshelf: uploaded book cards + upload form.
#[component]
fn BookshelfPage() -> impl IntoView {
    let books: Vec<BookInfo> = fixture_books();
    view! {
        <section class="page page--epub">
            <header class="page__header">
                <h2>"EPUB 书架"</h2>
                <p class="page__hint">"Upload a `.epub` file to begin reading."</p>
            </header>
            <Bookshelf books=books />
            <UploadForm />
        </section>
    }
}

/// EPUB reader page: takes `name` from the router param and
/// passes a fixture progress percentage into `<BookReader/>`.
#[component]
fn ReaderPage() -> impl IntoView {
    let params = use_params_map();
    let safe_name = move || {
        params
            .with(|m| m.get("name"))
            .unwrap_or_else(|| "unknown-book.epub".to_string())
    };
    let progress = move || fixture_book_progress(&safe_name());
    view! {
        <section class="page page--reader">
            <BookReader safe_name=safe_name() percent=progress() />
        </section>
    }
}

/// Root application component with top-bar nav + routed body.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="app-root">
                <nav class="app-nav">
                    <A href="/">"Articles"</A>
                    <span>" · "</span>
                    <A href="/feeds-ui">"Feeds"</A>
                    <span>" · "</span>
                    <A href="/epub">"EPUB"</A>
                    <span>" · "</span>
                    <A href="/settings-ui">"Settings"</A>
                </nav>
                <header class="app-header">
                    <h1>"miniflux-reader-rs"</h1>
                    <p class="tagline">"Rust + Leptos 0.7 unified reading platform v0.2.0"</p>
                </header>
                <main class="app-main">
                    <Routes fallback=|| {
                        view! {
                            <div class="route-fallback">
                                <h3>"Page not found"</h3>
                                <A href="/">"← Return home"</A>
                            </div>
                        }
                    }>
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/feeds-ui") view=FeedsPage />
                        <Route path=path!("/article/:id") view=ArticlePage />
                        <Route path=path!("/settings-ui") view=SettingsPageWrapper />
                        <Route path=path!("/epub") view=BookshelfPage />
                        <Route path=path!("/epub/read/:name") view=ReaderPage />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
