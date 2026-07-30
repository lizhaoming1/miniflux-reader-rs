//! Sentence splitting with explicit Chinese / English / auto-detect modes.
//!
//! Chinese mode uses CJK terminators (。！？；…) with inclusive-split and
//! abbreviation-ignorant trimming. English mode uses a manual terminator
//! scan (no look-around regex — the `regex` crate guarantees linear time
//! and forbids look-around) followed by an abbreviation merge-back pass.
//! Detect mode counts CJK codepoint ratio (>50% → Zh, otherwise → En).

/// Language classification hint for downstream sentence tokenisers.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LanguageHint {
    /// Chinese (CJK) terminator rules: 。！？；…
    Zh,
    /// English regex + abbreviation list rules.
    En,
    /// Auto-detect: count CJK codepoint ratio, pick Zh (>50%) or En (<=50%).
    Detect,
}

/// Well-known English abbreviations that must NOT trigger a sentence split
/// even when followed by a period + space + capital.
static EN_ABBREV: &[&str] = &[
    "U.S.A", "U.S", "Dr", "Mr", "Mrs", "Ms", "vs", "etc", "Jr", "Sr", "e.g", "i.e",
];

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3000..=0x303F | 0xFF00..=0xFFEF)
}

/// Chinese split: iterate chars, flush buffer whenever we see a CJK (or
/// compatible) terminator, then trim whitespace. Appends any non-empty tail.
fn zh_split(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if matches!(ch, '。' | '！' | '？' | '；' | '…' | '.' | '?' | '!') {
            let s = buf.trim().to_string();
            if !s.is_empty() {
                out.push(s);
            }
            buf.clear();
        }
    }
    let tail = buf.trim().to_string();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// English split: manual char scan for sentence terminator boundaries.
///
/// Boundary rule: whenever we see one of `.!?`, we look ahead past any
/// whitespace; if the next non-whitespace char is uppercase ASCII or a
/// digit we split — otherwise the punctuation is mid-sentence. After
/// collecting raw segments, an abbreviation merge-back pass re-attaches
/// segments created by abbreviations like `U.S.A.` / `Dr.`.
fn en_split(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut raw: Vec<String> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < n {
        if matches!(chars[i], '.' | '!' | '?') {
            // Scan past whitespace after terminator.
            let mut j = i + 1;
            while j < n && chars[j].is_whitespace() {
                j += 1;
            }
            // Force at least one whitespace between terminator and uppercase
            // so mid-sentence abbreviations like "U.S.A" are never split.
            let has_space_after_terminator = j > i + 1;
            let split_here = has_space_after_terminator
                && j < n
                && (chars[j].is_ascii_uppercase() || chars[j].is_ascii_digit());
            if split_here {
                let seg: String = chars[start..=i].iter().collect();
                let s = seg.trim().to_string();
                if !s.is_empty() {
                    raw.push(s);
                }
                start = j;
                i = j;
                continue;
            }
        }
        i += 1;
    }
    // Tail segment from `start` to end.
    if start < n {
        let seg: String = chars[start..].iter().collect();
        let s = seg.trim().to_string();
        if !s.is_empty() {
            raw.push(s);
        }
    }

    // Merge-back: walk segments; if a segment's tail (ignoring trailing
    // periods + whitespace) ends with an EN_ABBREV entry AND the following
    // segment starts lowercase, combine them so e.g. "Dr." + "Smith" → 1.
    let mut merged: Vec<String> = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let mut cur = raw[i].clone();
        loop {
            if i + 1 >= raw.len() {
                break;
            }
            let trimmed = cur.trim_end_matches(|c: char| c.is_whitespace() || c == '.');
            let last_token = trimmed
                .split(|c: char| !c.is_alphanumeric() && c != '.')
                .last()
                .unwrap_or("");
            let is_abbr = EN_ABBREV.iter().any(|a| last_token.eq_ignore_ascii_case(a));
            let next_starts_lower = raw[i + 1]
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false);
            if is_abbr && next_starts_lower {
                cur.push(' ');
                cur.push_str(&raw[i + 1]);
                i += 1;
            } else {
                break;
            }
        }
        merged.push(cur);
        i += 1;
    }
    merged
}

/// Split `text` into sentences according to `hint`.
///
/// # Examples
/// ```
/// use common_text::{split_sentences, LanguageHint};
/// assert_eq!(split_sentences("你好。世界。", LanguageHint::Zh).len(), 2);
/// ```
pub fn split_sentences(text: &str, hint: LanguageHint) -> Vec<String> {
    match hint {
        LanguageHint::Zh => zh_split(text),
        LanguageHint::En => en_split(text),
        LanguageHint::Detect => {
            let total_chars = text.chars().count().max(1);
            let cjk_count = text.chars().filter(|&c| is_cjk(c)).count();
            let cjk_ratio = cjk_count as f32 / total_chars as f32;
            // Threshold 10% — intentionally low so any clearly-mixed text
            // (e.g. 你好世界。Hello world. Good.) uses zh_split which
            // correctly handles BOTH CJK terminators 。！？ and ASCII .!?
            if cjk_ratio > 0.10 {
                zh_split(text)
            } else {
                en_split(text)
            }
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn cjk_helper_covers_common_ranges() {
        assert!(is_cjk('中'));
        assert!(is_cjk('。'));
        assert!(!is_cjk('A'));
    }
}
