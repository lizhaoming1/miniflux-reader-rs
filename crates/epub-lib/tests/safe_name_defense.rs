//! PR#3 Task 3.2 — RED phase: directory-traversal defense in `safe_name_for`.

use epub_lib::safe_name_for;

#[test]
fn rejects_dotdot_traversal() {
    let safe = safe_name_for("../../etc/passwd");
    assert!(
        !safe.contains(".."),
        "safe name must not contain '..': got {safe}"
    );
    assert!(
        !safe.contains('/') && !safe.contains('\\'),
        "safe name must not contain path separators: got {safe}"
    );
}

#[test]
fn strips_path_and_keeps_basename_for_normal_file() {
    let safe = safe_name_for("my book.epub");
    assert!(
        safe.contains("my") && safe.contains("book"),
        "normal filename should preserve readable tokens: got {safe}"
    );
    assert!(!safe.is_empty());
    assert!(
        !safe.contains('/') && !safe.contains('\\'),
        "no path separators allowed: got {safe}"
    );
}

#[test]
fn rejects_absolute_path() {
    let safe = safe_name_for("/etc/shadow");
    assert!(
        !safe.starts_with('/'),
        "absolute path must not leak leading slash: got {safe}"
    );
}
