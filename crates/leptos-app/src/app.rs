//! `<App/>` — root component with leptos_router.

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes, A},
    path,
};

/// Root application component with top-bar nav + routed body.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="app-root">
                <nav class="app-nav">
                    <A href="/">"Articles"</A>
                    <span>" · "</span>
                    <A href="/feeds">"Feeds"</A>
                    <span>" · "</span>
                    <A href="/epub">"EPUB"</A>
                    <span>" · "</span>
                    <A href="/settings">"Settings"</A>
                </nav>
                <header class="app-header">
                    <h1>"miniflux-reader-rs"</h1>
                    <p class="tagline">"Rust + Leptos 0.7 unified reading platform v0.2.0"</p>
                </header>
                <main class="app-main">
                    <Routes fallback=|| view! { <div class="route-fallback">"Loading…"</div> }>
                        <Route path=path!("/") view=|| view! { <div class="home">"Article list"</div> }/>
                        <Route path=path!("/feeds") view=|| view! { <div class="feeds-page">"Feeds page"</div> }/>
                        <Route path=path!("/article/:id") view=|| view! { <div class="article-page">"Article page"</div> }/>
                        <Route path=path!("/settings") view=|| view! { <div class="settings-page">"Settings page"</div> }/>
                        <Route path=path!("/epub") view=|| view! { <div class="epub-page">"Bookshelf"</div> }/>
                        <Route path=path!("/epub/read/:name") view=|| view! { <div class="reader-page">"Reader"</div> }/>
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
