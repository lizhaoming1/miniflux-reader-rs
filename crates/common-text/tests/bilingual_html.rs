//! PR#2 Task 2.3 — RED phase: 2 failing tests for bilingual HTML renderer.
//!
//! Scaffold `render_bilingual_div` returns `""`, so the non-empty test fails
//! with substring-missing assertions.

use common_text::{render_bilingual_div, BilingualSentence};

#[test]
fn single_pair_renders_src_p_and_zh_paragraph_with_blue_border() {
    let pair = BilingualSentence {
        src: "Hello world.".to_string(),
        zh: "你好世界。".to_string(),
        src_start: 0,
        src_end: 12,
    };
    let html = render_bilingual_div(&[pair]);
    // Source paragraph: exact text content.
    assert!(
        html.contains("<p>Hello world.</p>"),
        "src <p> tag with exact content missing, got: {}",
        html
    );
    // Zh paragraph: must have `class="sentence--zh"` AND the inline blue
    // `border-left:3px solid #3f87ff` style AND the Chinese text.
    assert!(
        html.contains(r#"class="sentence--zh""#),
        "zh <p> missing sentence--zh class, got: {}",
        html
    );
    assert!(
        html.contains("border-left:3px solid #3f87ff"),
        "zh <p> missing blue border-left style, got: {}",
        html
    );
    assert!(
        html.contains("你好世界。"),
        "zh text content missing, got: {}",
        html
    );
    // Outer wrapper div class="sentence".
    assert!(
        html.contains(r#"<div class="sentence">"#),
        "outer wrapper missing sentence class, got: {}",
        html
    );
}

#[test]
fn empty_slice_returns_empty_string() {
    let html = render_bilingual_div(&[]);
    assert_eq!(html, "", "no bilingual pairs → blank HTML, not empty div");
}
