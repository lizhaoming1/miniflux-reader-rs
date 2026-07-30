//! tracing initialisation.

use tracing_subscriber::EnvFilter;

/// Initialise the global tracing subscriber using the `RUST_LOG` env var,
/// falling back to `info` if unset. Safe to call multiple times — subsequent
/// calls are no-ops.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

/// Initialise the global tracing subscriber with an explicit filter string.
/// Safe to call multiple times — subsequent calls are no-ops.
pub fn init_tracing_with_filter(filter: &str) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .try_init();
}
