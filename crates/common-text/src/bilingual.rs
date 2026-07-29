//! Scaffold only — real implementation added in PR#2.

use serde::{Deserialize, Serialize};

/// (src, zh) pair used to render the bilingual DOM. Real struct derives
/// more traits and includes offset info in PR#2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilingualSentence {
    pub src: String,
    pub zh:  String,
}

/// Renders the bilingual `<div class=sentence><p>src</p><p class=sentence--zh>…</p></div>`.
/// Real renderer builds proper `String` with CSS border-left in PR#2.
pub fn render_bilingual_div(_sentences: &[BilingualSentence]) -> String {
    String::new()
}
