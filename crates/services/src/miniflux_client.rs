//! Miniflux HTTP client: login form, session token, and client stub.
//!
//! The real `login()` / `fetch_page()` / `set_cookies()` wiring lands in a
//! later PR; this module only exposes the request/response shapes plus a
//! placeholder client so other crates can compile against the API.

use serde::{Deserialize, Serialize};

/// Credentials submitted to the Miniflux `/login` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoginForm {
    /// Miniflux account username.
    pub username: String,
    /// Miniflux account password (sent over TLS in production).
    pub password: String,
}

/// An authenticated Miniflux session, typically a session cookie value.
#[derive(Debug, Clone, Default)]
pub struct MinifluxSession(pub String);

/// Placeholder Miniflux HTTP client.
///
/// Real `login()` / `fetch_page()` / `set_cookies()` implementations land in
/// a later PR; for now callers can only construct a default stub.
#[derive(Clone, Default)]
pub struct MinifluxClient;

impl MinifluxClient {
    /// Construct a placeholder client with no configured base URL or cookies.
    pub fn placeholder() -> Self {
        Self
    }
}
