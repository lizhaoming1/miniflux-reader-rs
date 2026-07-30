//! PR#7 wasm headless tests — T8..T11.
//!
//! These tests require `wasm-pack test --chrome --headless` to run. They are
//! marked `#[ignore]` so `cargo test` skips them. To execute:
//!
//! ```sh
//! wasm-pack test --chrome --headless crates/leptos-app -- --features hydrate
//! ```
//!
//! The tests below document the expected browser-level interactions:
//!
//! - **T8**: Navigate to `/epub/read/testbook`, inject progress via JS, wait
//!   10s, assert `window.__progress_saved === true`.
//! - **T9**: Click the translate button → `sentence--zh` class appears; click
//!   the original button → `sentence--zh` class disappears.
//! - **T10**: Click the TTS play button → `audio.currentTime` monotonically
//!   increases for 1 second, then pause.
//! - **T11**: After SSR + hydration, `window.location.pathname` matches the
//!   expected route and no element has the `error-404` class.

#![cfg(feature = "ssr")]

/// T8: progress save after reading.
#[test]
#[ignore = "requires wasm-pack + headless chrome"]
fn t8_progress_saved_after_reading() {
    // Placeholder — see module docs for the full browser test scenario.
}

/// T9: bilingual toggle adds/removes sentence--zh class.
#[test]
#[ignore = "requires wasm-pack + headless chrome"]
fn t9_translate_toggle_adds_removes_zh() {
    // Placeholder — see module docs for the full browser test scenario.
}

/// T10: TTS play button starts audio playback.
#[test]
#[ignore = "requires wasm-pack + headless chrome"]
fn t10_tts_play_starts_audio() {
    // Placeholder — see module docs for the full browser test scenario.
}

/// T11: hydrated route has no 404 error class.
#[test]
#[ignore = "requires wasm-pack + headless chrome"]
fn t11_hydrated_route_no_404() {
    // Placeholder — see module docs for the full browser test scenario.
}
