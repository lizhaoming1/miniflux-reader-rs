//! Runtime settings UI: 5 input fields (poll/timeout/UA/target_lang/voice)
//! with a single save button that PUTs JSON to /settings.

use leptos::prelude::*;

/// Form showing all runtime-overridable settings, save → PUT /settings.
#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <form class="settings-form" method="PUT" action="/settings" enctype="application/json">
            <fieldset>
                <legend>"Feed engine"</legend>
                <label>
                    "Poll interval (seconds):"
                    <input type="number" name="feed.poll_interval_secs" min="30" step="1" value="900"/>
                </label>
                <label>
                    "Fetch timeout (seconds):"
                    <input type="number" name="feed.fetch_timeout_secs" min="1" step="1" value="30"/>
                </label>
                <label>
                    "User-Agent:"
                    <input type="text" name="feed.user_agent" value="miniflux-reader-rs/0.2.0"/>
                </label>
            </fieldset>
            <fieldset>
                <legend>"Services"</legend>
                <label>
                    "Translate target language:"
                    <input type="text" name="translate.target_lang" value="zh-CN"/>
                </label>
                <label>
                    "TTS voice:"
                    <input type="text" name="tts.voice" value="zh-CN-XiaoxiaoNeural"/>
                </label>
            </fieldset>
            <button type="submit">"Save settings"</button>
        </form>
    }
}
