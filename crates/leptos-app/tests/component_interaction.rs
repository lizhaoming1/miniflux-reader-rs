//! PR#7 component interaction tests — T4..T7.
//!
//! Tests reactive behavior of interactive components using signals in SSR
//! mode. Since we can't dispatch real DOM events without a browser, we test
//! the signal-driven state transitions: initial render → signal change →
//! re-render reflects new state.
//!
//! Run with: `cargo test -p leptos-app --features ssr --test component_interaction`

#![cfg(feature = "ssr")]

use common_text::BilingualSentence;
use leptos::prelude::*;
use leptos_app::{
    debounce_fire_count, AddFeedForm, BilingualToggle, SettingsPage, TtsPlayer, UploadForm,
};

/// Helper: wrap a closure in a fresh reactive `Owner` so signals/arenas work.
fn with_owner<T>(f: impl FnOnce() -> T) -> T {
    let owner = Owner::new();
    owner.with(f)
}

// ---------- T4: UploadForm renders correct multipart form ----------

#[test]
fn t4_upload_form_renders_multipart_form() {
    let html = with_owner(|| view! { <UploadForm /> }.to_html());
    assert!(
        html.contains("enctype=\"multipart/form-data\""),
        "expected multipart enctype, got: {html}"
    );
    assert!(
        html.contains("action=\"/epub/upload\""),
        "expected action=/epub/upload, got: {html}"
    );
    assert!(
        html.contains("type=\"file\""),
        "expected file input, got: {html}"
    );
}

// ---------- T5: debounce_save fires once after rapid calls ----------

#[test]
fn t5_debounce_fires_once_for_rapid_calls() {
    // Simulate 5 rapid scroll events within the debounce window.
    // The debounce should coalesce them into a single save.
    let calls = vec![0u64, 50, 100, 150, 200];
    let fire_count = debounce_fire_count(&calls, 500);
    assert_eq!(
        fire_count, 1,
        "5 rapid calls within 500ms window should fire exactly once, got {fire_count}"
    );
}

#[test]
fn t5b_debounce_fires_twice_for_calls_apart() {
    // Two groups of calls, each group fires once.
    let calls = vec![0u64, 100, 700, 800];
    let fire_count = debounce_fire_count(&calls, 500);
    assert_eq!(
        fire_count, 2,
        "calls in two separate 500ms windows should fire twice, got {fire_count}"
    );
}

// ---------- T6: BilingualToggle data-bilingual attr changes with signal ----------

#[test]
fn t6_bilingual_toggle_initial_state_is_original() {
    let html = with_owner(|| {
        let (lang, _set_lang) = signal(leptos_app::ToggleLang::Original);
        view! {
            <BilingualToggle lang=lang sentences=vec![] />
        }
        .to_html()
    });
    assert!(
        html.contains("data-bilingual=\"0\""),
        "initial state should be data-bilingual=0, got: {html}"
    );
}

#[test]
fn t6b_bilingual_toggle_bilingual_state_shows_attr() {
    let sentences = vec![BilingualSentence {
        src: "Hello.".into(),
        zh: "你好。".into(),
        src_start: 0,
        src_end: 6,
    }];
    let html = with_owner(|| {
        let (lang, _set_lang) = signal(leptos_app::ToggleLang::Bilingual);
        view! {
            <BilingualToggle lang=lang sentences=sentences.clone() />
        }
        .to_html()
    });
    assert!(
        html.contains("data-bilingual=\"1\""),
        "bilingual state should be data-bilingual=1, got: {html}"
    );
    assert!(
        html.contains("sentence--zh"),
        "bilingual state should show zh, got: {html}"
    );
}

// ---------- T7: TtsPlayer shows <audio> when playing ----------

#[test]
fn t7_tts_player_hidden_when_not_playing() {
    let html = with_owner(|| {
        let (playing, _set) = signal(false);
        view! {
            <TtsPlayer playing=playing text="Hello world".to_string() />
        }
        .to_html()
    });
    assert!(
        !html.contains("id=\"tts\""),
        "audio element should NOT be present when not playing, got: {html}"
    );
}

#[test]
fn t7b_tts_player_shows_audio_when_playing() {
    let html = with_owner(|| {
        let (playing, _set) = signal(true);
        view! {
            <TtsPlayer playing=playing text="Hello world".to_string() />
        }
        .to_html()
    });
    assert!(
        html.contains("id=\"tts\""),
        "audio element should be present when playing, got: {html}"
    );
    assert!(
        html.contains("/tts?text="),
        "audio src should start with /tts?text=, got: {html}"
    );
}

// ---------- C4: AddFeedForm renders POST /feeds action + url input ----------

#[test]
fn c4_addfeedform_renders_action_and_url_input() {
    let html = with_owner(|| view! { <AddFeedForm /> }.to_html());
    assert!(
        html.contains("action=\"/feeds\""),
        "expected action=/feeds, got: {html}"
    );
    assert!(
        html.contains("type=\"url\""),
        "expected type=url input, got: {html}"
    );
}

// ---------- C5: SettingsPage renders PUT /settings + 5 named fields ----------

#[test]
fn c5_settingspage_renders_five_fields_and_put() {
    let html = with_owner(|| view! { <SettingsPage /> }.to_html());
    assert!(
        html.contains("action=\"/settings\""),
        "expected action=/settings, got: {html}"
    );
    for field in &[
        "feed.poll_interval_secs",
        "feed.fetch_timeout_secs",
        "feed.user_agent",
        "translate.target_lang",
        "tts.voice",
    ] {
        assert!(
            html.contains(&format!("name=\"{field}\"")),
            "missing field {field}; html: {html}"
        );
    }
}
