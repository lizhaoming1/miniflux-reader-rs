//! Article list view (news headlines from a feed or all feeds).

use leptos::prelude::*;

use crate::ArticleSummary;

/// Renders rows of article summaries with read/title/published/feed metadata.
#[component]
pub fn ArticleList(articles: Vec<ArticleSummary>) -> impl IntoView {
    if articles.is_empty() {
        return view! {
            <div class="article-list empty">
                <p class="empty-state">"No articles — add a feed or trigger a sync."</p>
            </div>
        }
        .into_any();
    }
    view! {
        <ol class="article-list">
            <For each=move || articles.clone() key=|a| a.id let:a>
                <li class="article-row" class:is-read=a.read>
                    <a class="article-title" href=format!("/article/{}", a.id)>{a.title.clone()}</a>
                    <span class="article-meta">
                        {format!("{} · {}", a.feed_title, a.published_at)}
                    </span>
                </li>
            </For>
        </ol>
    }
    .into_any()
}
