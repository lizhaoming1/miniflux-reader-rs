//! Scaffold only — real <App/>, <Bookshelf/>, <BookReader/>, <MinifluxArticle/>
//! + server fn, error boundary ALL added in PR#7 (TDD: RED SSR render tests first).

pub mod app {
    /// Placeholder App. Real Leptos view! { <Router> ... routes ... </Router> } in PR#7.
    #[leptos::component]
    pub fn App() -> impl leptos::IntoView { () }
}
pub use app::App;

pub mod bookshelf    {}
pub mod book_reader  {}
pub mod miniflux_article {}
pub mod server_fns   {}
pub mod error_boundary {}
