// Scaffold entry point. PR#1 only verifies the workspace links,
// crates compile, and 1 placeholder test passes. The real wiring
// happens in PR#6 (http-server routes + state + proxy layer).
fn main() {
    eprintln!(
        "http-server: PR#1 scaffold — run `cargo test --workspace` to\n\
         verify 1 placeholder test passes. Real binary entry point\n\
         (axum server with Leptos SSR + Hydration + Tower catch-all\n\
         MinifluxProxyLayer) is added in PR#6."
    );
}
