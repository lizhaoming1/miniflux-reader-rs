//! `<Bookshelf/>` — renders the list of uploaded EPUBs as cards.
//!
//! Empty state shows a `暂无 EPUB` placeholder. Non-empty state renders one
//! `<article class="book-card">` per book with title + author.

use leptos::prelude::*;

use crate::BookInfo;

/// Bookshelf view: shows all uploaded EPUBs, or a placeholder when empty.
#[component]
pub fn Bookshelf(books: Vec<BookInfo>) -> impl IntoView {
    if books.is_empty() {
        view! {
            <div class="bookshelf bookshelf--empty">
                <p class="bookshelf__placeholder">"暂无 EPUB"</p>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="bookshelf">
                <For each=move || books.clone() key=|b| b.safe_name.clone() let:book>
                    <article class="book-card">
                        <h3 class="book-card__title">{book.title.clone()}</h3>
                        <p class="book-card__author">{book.author.clone()}</p>
                        <a href=format!("/epub/read/{}", book.safe_name)>"阅读"</a>
                    </article>
                </For>
            </div>
        }
        .into_any()
    }
}

/// Upload form for EPUB files. POSTs a multipart body to `/epub/upload`.
#[component]
pub fn UploadForm() -> impl IntoView {
    view! {
        <form class="upload-form" action="/epub/upload" method="POST" enctype="multipart/form-data">
            <input type="file" name="file" accept=".epub" />
            <button type="submit">"上传 EPUB"</button>
        </form>
    }
}
