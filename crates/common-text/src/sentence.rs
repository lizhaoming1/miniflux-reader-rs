//! Scaffold only — real implementation added in PR#2 (RED tests first).

/// Placeholder enum. Real implementation (Zh / En / Detect) in PR#2.
pub enum LanguageHint {
    Detect,
}

/// Placeholder function. Real sentence split (中英混合分句，pysbd port) in PR#2.
pub fn split_sentences(_text: &str, _hint: LanguageHint) -> Vec<String> {
    vec![]
}
