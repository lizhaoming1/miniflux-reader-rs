//! Feed subscription UI: list of feeds with unread counts, an add form,
//! and OPML import/export buttons.

use leptos::prelude::*;

use crate::FeedInfo;

/// Renders a list of feed cards showing title, site, unread count, delete button.
#[component]
pub fn FeedList(feeds: Vec<FeedInfo>) -> impl IntoView {
    if feeds.is_empty() {
        return view! {
            <div class="feed-list empty">
                <p class="empty-state">"No feeds yet. Add one below."</p>
            </div>
        }
        .into_any();
    }
    view! {
        <ul class="feed-list">
            <For each=move || feeds.clone() key=|f| f.id let:f>
                <li class="feed-card">
                    <a class="feed-title" href=format!("/feeds/{}", f.id)>{f.title.clone()}</a>
                    <span class="feed-unread">{f.unread_count}</span>
                    <button class="feed-delete" data-feed-id=f.id>"Delete"</button>
                </li>
            </For>
        </ul>
    }
    .into_any()
}

/// One-input form that POSTs `{"url": "..."}` to `/feeds`.
#[component]
pub fn AddFeedForm() -> impl IntoView {
    view! {
        <form class="add-feed-form" method="POST" action="/feeds" enctype="application/json">
            <label>
                "Feed URL:"
                <input type="url" name="url" required placeholder="https://example.com/feed.xml"/>
            </label>
            <button type="submit">"Add feed"</button>
        </form>
    }
}

/// OPML import (file picker → POST text body) and export anchor.
#[component]
pub fn OpmlActions() -> impl IntoView {
    view! {
        <div class="opml-actions">
            <form class="opml-import" method="POST" action="/opml/import" enctype="text/plain">
                <input type="file" accept=".opml,text/xml,application/xml" name="opml" required/>
                <button type="submit">"Import OPML"</button>
            </form>
            <a class="opml-export" href="/opml/export" download="subscriptions.opml">"Export OPML"</a>
        </div>
    }
}
