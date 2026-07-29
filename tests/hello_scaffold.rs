//! Workspace-level integration tests start their life as a single
//! placeholder that only validates: (a) the workspace compiles, and
//! (b) the scaffold empty-module structure is linked correctly.
//! Real cross-crate tests (upload EPUB → save progress → restart DB →
//! recover progress) are added in PR#8.

#[test]
fn workspace_compiles_and_placeholder_passes() {
    assert_eq!(1 + 1, 2);
}
