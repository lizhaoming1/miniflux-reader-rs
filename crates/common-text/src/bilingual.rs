//! Bilingual (source / zh-translation) pair struct and HTML renderer.
//!
//! The renderer builds per-sentence `<div class="sentence">` blocks with
//! an inline-blue-style zh paragraph so miniflux proxy injection can
//! colourise translated content without requiring external CSS.

use serde::{Deserialize, Serialize};

/// A bilingual (source / translated-zh) sentence pair with byte offsets
/// into the original concatenated source text. Offsets match Rust `&str`
/// slicing conventions (start inclusive, end exclusive) and are used by
/// downstream layers (EPUB reader, miniflux DOM-injection proxy) for TTS
/// highlight sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BilingualSentence {
    /// Original-language sentence (exact slice of the source text).
    pub src: String,
    /// Chinese translation produced by `services::TranslateService`.
    pub zh: String,
    /// Inclusive byte start offset into the concatenated original text.
    pub src_start: usize,
    /// Exclusive byte end offset into the concatenated original text.
    pub src_end: usize,
}

/// Inline style for the zh `<p>` — a blue left-border accent plus padding
/// and a deeper-blue text colour — matched exactly to tests and to the
/// Python implementation's `sentence--zh` CSS class.
const ZH_P_STYLE: &str = "border-left:3px solid #3f87ff;padding-left:8px;color:#0066cc;";

/// Render `sentences` as HTML. Empty input → empty output (no wrapper div).
///
/// Each non-empty sentence emits:
/// ```html
/// <div class="sentence"><p>$src</p><p class="sentence--zh" style="…">$zh</p></div>
/// ```
/// Concatenated in order with no separator.
pub fn render_bilingual_div(sentences: &[BilingualSentence]) -> String {
    let mut out = String::with_capacity(sentences.len() * 160);
    for s in sentences {
        out.push_str(r#"<div class="sentence">"#);
        out.push_str("<p>");
        out.push_str(&s.src);
        out.push_str("</p>");
        out.push_str(r#"<p class="sentence--zh" style=""#);
        out.push_str(ZH_P_STYLE);
        out.push_str(r#"">"#);
        out.push_str(&s.zh);
        out.push_str("</p>");
        out.push_str("</div>");
    }
    out
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn two_sentences_concatenate_two_wrappers() {
        let s1 = BilingualSentence {
            src: "A.".into(),
            zh: "甲。".into(),
            src_start: 0,
            src_end: 2,
        };
        let s2 = BilingualSentence {
            src: "B.".into(),
            zh: "乙。".into(),
            src_start: 2,
            src_end: 4,
        };
        let html = render_bilingual_div(&[s1, s2]);
        assert_eq!(
            html.matches(r#"<div class="sentence">"#).count(),
            2,
            "N sentences → N wrapper divs"
        );
    }
}
