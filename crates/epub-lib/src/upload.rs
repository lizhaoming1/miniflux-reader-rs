//! Upload helpers: safe-name derivation + disk persistence.
//!
//! `safe_name_for` strips path components and rejects directory traversal so
//! user-supplied filenames can never escape the books directory.
//! `save_upload_to_disk` writes the bytes to `rust-epub-books/` (under the
//! system temp dir unless overridden by `RUST_EPUB_BOOKS_DIR`) and returns
//! the safe name.

use std::path::PathBuf;

use crate::error::{EpubError, Result};

/// Default books directory (under temp dir). Overridable via
/// `RUST_EPUB_BOOKS_DIR` env var — used by tests and the http-server
/// upload route.
fn books_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("RUST_EPUB_BOOKS_DIR") {
        return PathBuf::from(dir);
    }
    std::env::temp_dir().join("rust-epub-books-test")
}

/// Derive a filesystem-safe name from a user-supplied filename.
///
/// Rules:
/// 1. Split on `/` and `\`, take the last component (basename only).
/// 2. Reject if the result contains `..` (directory traversal) — sanitise
///    by replacing `..` with `_`.
/// 3. Replace NUL bytes and other control characters with `_`.
/// 4. If the result is empty after sanitisation, return `unnamed`.
pub fn safe_name_for(file_name: &str) -> String {
    // 1. Basename only — strip any path components.
    let basename = file_name
        .replace('\\', "/")
        .split('/')
        .rfind(|s| !s.is_empty() && s != &".".to_string())
        .unwrap_or("")
        .to_string();

    // 2. Sanitise: replace `..` and path separators and control chars.
    let mut safe = String::with_capacity(basename.len());
    for ch in basename.chars() {
        if ch == '.' {
            safe.push(ch);
        } else if ch.is_control() || ch == '/' || ch == '\\' || ch == '\0' {
            safe.push('_');
        } else {
            safe.push(ch);
        }
    }
    // Collapse any `..` sequences (e.g. "foo..bar" → "foo__bar" is fine;
    // but ".." alone → "__" which is safe).
    if safe.contains("..") {
        safe = safe.replace("..", "_");
    }

    // 3. Strip leading dots/underscores that could still be tricky.
    while safe.starts_with('.') || safe.starts_with('_') {
        safe.remove(0);
    }

    if safe.is_empty() {
        "unnamed".to_string()
    } else {
        safe
    }
}

/// Persist `bytes` to `<books_dir>/<safe_name>` and return the safe name.
/// The `books_dir` is created if it does not exist.
pub fn save_upload_to_disk(bytes: &[u8], file_name: &str) -> Result<String> {
    let safe = safe_name_for(file_name);
    let dir = books_dir();
    std::fs::create_dir_all(&dir)?;
    let path: PathBuf = dir.join(&safe);
    std::fs::write(&path, bytes)?;
    Ok(safe)
}

/// Resolve the on-disk path for a given safe name (used by callers that
/// need to reopen the file later).
#[allow(dead_code)]
pub fn path_for_safe_name(safe_name: &str) -> PathBuf {
    books_dir().join(safe_name)
}

/// Ensure the marker variant is reachable (prevents dead-code warnings in
/// scaffold state).
#[allow(dead_code)]
fn _ensure_unreachable_marker() -> EpubError {
    EpubError::UnsafeFilename(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basename_only_for_normal_filename() {
        assert_eq!(safe_name_for("book.epub"), "book.epub");
    }

    #[test]
    fn strips_directory_components() {
        assert_eq!(safe_name_for("a/b/c.epub"), "c.epub");
        assert_eq!(safe_name_for("a\\b\\c.epub"), "c.epub");
    }
}
