//! `<BookReader/>` — EPUB reader view with progress tracking.
//!
//! Renders a reader container that carries the current scroll percent as a
//! `data-percent` attribute (used by the hydration layer + TTS highlight).

use leptos::prelude::*;

/// EPUB reader view. `percent` is rendered into `data-percent` so the
/// hydration layer and tests can read it off the DOM.
#[component]
pub fn BookReader(safe_name: String, percent: f64) -> impl IntoView {
    let pct = percent as i32;
    let name_for_attr = safe_name.clone();
    view! {
        <div class="book-reader" data-percent=pct.to_string()>
            <div class="book-reader__header">
                <a href="/epub">"← 返回书架"</a>
                <span class="book-reader__title">{safe_name.clone()}</span>
            </div>
            <div class="book-reader__content" data-book=name_for_attr>
                <p>"Loading book content for " {safe_name} "…"</p>
            </div>
        </div>
    }
}

/// TTS audio player. When `playing` is `true`, renders an `<audio id="tts">`
/// element with `src="/tts?text=<text>"`.
#[component]
pub fn TtsPlayer(playing: ReadSignal<bool>, text: String) -> impl IntoView {
    view! {
        <div class="tts-player">
            <Show when=move || playing.get()>
                <audio id="tts" src=format!("/tts?text={}", text) controls />
            </Show>
        </div>
    }
}
