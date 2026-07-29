//! # leptos-app
//!
//! Holds the three UI components that the MVP ships:
//!   * `<Bookshelf />`   — grid of book cards + upload drop zone
//!   * `<BookReader />`  — per-chapter renderer, scroll->progress save,
//!                        bilingual-sentence toggle
//!   * `<MinifluxArticle/>` — wraps the HTML the proxy layer injected,
//!                        adds a 原文/双语 toggle row + TTS play bar
//! Server functions (the `#[server]` items) are declared here and
//! automatically wired by `leptos_axum::generate_route_list` from the
//! axum binary crate.

#![cfg_attr(feature = "csr", allow(unused_imports))]

pub mod app;
pub mod bookshelf;
pub mod book_reader;
pub mod miniflux_article;
pub mod server_fns;
pub mod error_boundary;

pub use app::*;
