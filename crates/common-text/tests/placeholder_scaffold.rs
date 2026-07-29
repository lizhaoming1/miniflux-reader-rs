//! Scaffold-level placeholder. PR#1's only deliverable for the test
//! surface is: (a) the workspace compiles (`cargo check` above) and
//! (b) `cargo test` runs this single placeholder test and returns 1 PASS.
//!
//! All real unit / integration tests are added by the RED-first commits
//! of PR#2 through PR#8.

#[test]
fn workspace_placeholder_exactly_one_assumption_holds() {
    assert_eq!(2, 1 + 1);
}
