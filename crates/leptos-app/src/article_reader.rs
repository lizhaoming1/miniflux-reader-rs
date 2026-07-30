//! `<ArticleReader/>` — renders an article body with optional bilingual
//! translation.
//!
//! When `lang` is [`ToggleLang::Original`], only the source sentences are
//! rendered. When `lang` is [`ToggleLang::Bilingual`], each source sentence
//! is followed by its Chinese translation in a `<p class="sentence--zh">`.

use leptos::prelude::*;

use common_text::BilingualSentence;

use crate::ToggleLang;

/// Article view with toggleable bilingual rendering.
#[component]
pub fn ArticleReader(sentences: Vec<BilingualSentence>, lang: ToggleLang) -> impl IntoView {
    match lang {
        ToggleLang::Original => view! {
            <div class="article-reader" data-bilingual="0">
                <For each=move || sentences.clone() key=|s| s.src_start let:s>
                    <p class="sentence">{s.src.clone()}</p>
                </For>
            </div>
        }
        .into_any(),
        ToggleLang::Bilingual => view! {
            <div class="article-reader" data-bilingual="1">
                <For each=move || sentences.clone() key=|s| s.src_start let:s>
                    <p class="sentence">{s.src.clone()}</p>
                    <p class="sentence--zh">{s.zh.clone()}</p>
                </For>
            </div>
        }
        .into_any(),
    }
}

/// Reactive bilingual article that reads `lang` from a signal, allowing
/// runtime toggle between Original and Bilingual modes.
#[component]
pub fn BilingualToggle(
    lang: ReadSignal<ToggleLang>,
    sentences: Vec<BilingualSentence>,
) -> impl IntoView {
    view! {
        <div
            class="article-reader"
            data-bilingual=move || match lang.get() {
                ToggleLang::Original => "0",
                ToggleLang::Bilingual => "1",
            }
        >
            <For each=move || sentences.clone() key=|s| s.src_start let:s>
                <p class="sentence">{s.src.clone()}</p>
                <Show when=move || lang.get() == ToggleLang::Bilingual>
                    <p class="sentence--zh">{s.zh.clone()}</p>
                </Show>
            </For>
        </div>
    }
}
