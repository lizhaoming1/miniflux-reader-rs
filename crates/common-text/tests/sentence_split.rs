//! PR#2 Task 2.1 — RED phase: 5 failing tests for sentence splitting.
//!
//! These tests assert real behavior before any implementation exists.
//! The scaffold `split_sentences` currently returns `vec![]`, so every
//! assertion in this file should FAIL until we implement GREEN phase.

use common_text::*;

#[test]
fn zh_period_splits_two_clauses() {
    let r = split_sentences("你好。世界。", LanguageHint::Zh);
    assert_eq!(
        r,
        vec!["你好。".to_string(), "世界。".to_string()],
        "两个中文分句应该被句号正确分隔"
    );
}

#[test]
fn en_period_splits_two_sentences() {
    let r = split_sentences("Hello world. Good morning.", LanguageHint::En);
    assert_eq!(r.len(), 2, "英文两句话应该返回 2 个句子");
    assert_eq!(r[0], "Hello world.".to_string());
    assert_eq!(r[1], "Good morning.".to_string());
}

#[test]
fn detect_zh_mixed_en_handles_both() {
    let r = split_sentences("你好世界。Hello world. Good.", LanguageHint::Detect);
    assert!(
        r.len() >= 3,
        "Detect 模式下中英混合文本至少切出 3 句，实际 len = {}",
        r.len()
    );
}

#[test]
fn empty_text_returns_empty_vec() {
    assert_eq!(
        split_sentences("", LanguageHint::Detect).len(),
        0,
        "空字符串必须返回空 Vec"
    );
}

#[test]
fn abbreviations_without_space_do_not_split() {
    // "U.S.A is great." → 1 sentence, not 3
    let r = split_sentences("U.S.A is great.", LanguageHint::En);
    assert_eq!(
        r.len(),
        1,
        "U.S.A 缩写不能被误切为多句，实际 = {:?}",
        r
    );
}
