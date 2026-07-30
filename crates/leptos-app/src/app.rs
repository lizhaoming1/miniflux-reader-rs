//! `<App/>` — root component with router.
//!
//! Routes:
//! - `/` → redirect to `/epub`
//! - `/epub` → `<Bookshelf/>`
//! - `/epub/read/:name` → `<BookReader/>`

use leptos::prelude::*;

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app-root">
            <h1>"miniflux-reader-rs"</h1>
            <p>"Rust + Leptos 0.7 SSR MVP"</p>
        </div>
    }
}
