//! Miniflux HTTP client: login form, session token, and request forwarding.
//!
//! `MinifluxClient` wraps a `reqwest::Client` configured to talk to a single
//! Miniflux upstream. `login()` posts credentials and extracts the session
//! cookie; `forward()` proxies an arbitrary request to the upstream so the
//! Tower catch-all layer can transparently relay browser traffic.

use crate::error::{Result as SvcResult, ServiceError};
use serde::{Deserialize, Serialize};

/// Credentials submitted to the Miniflux `/login` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoginForm {
    /// Miniflux account username.
    pub username: String,
    /// Miniflux account password (sent over TLS in production).
    pub password: String,
}

/// An authenticated Miniflux session.
///
/// The inner `String` is the raw `name=value` cookie pair extracted from the
/// upstream `Set-Cookie` header (e.g. `MinifluxSession=abc123`).
#[derive(Debug, Clone, Default)]
pub struct MinifluxSession(pub String);

/// HTTP client for a single Miniflux upstream.
///
/// Holds the base URL and a reusable `reqwest::Client` (cookie store
/// disabled — the caller manages session cookies explicitly via
/// `forward()` headers).
#[derive(Clone)]
pub struct MinifluxClient {
    /// Base URL of the Miniflux server (no trailing slash).
    base_url: String,
    /// Reusable reqwest client (no cookie store).
    client: reqwest::Client,
}

impl Default for MinifluxClient {
    fn default() -> Self {
        Self::new("")
    }
}

impl MinifluxClient {
    /// Construct a client targeting `base_url` (trailing slash trimmed).
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .cookie_store(false)
                .build()
                .unwrap_or_default(),
        }
    }

    /// Construct a placeholder client with no configured base URL.
    pub fn placeholder() -> Self {
        Self::new("")
    }

    /// Return the configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// POST `form` to `{base_url}/login` and extract the session cookie.
    ///
    /// On success, returns [`MinifluxSession`] whose inner string is the
    /// `name=value` portion of the `Set-Cookie` response header. On non-2xx
    /// status, returns [`ServiceError::Upstream`]; on connection failure,
    /// returns [`ServiceError::Network`].
    pub async fn login(&self, form: &LoginForm) -> SvcResult<MinifluxSession> {
        let url = format!("{}/login", self.base_url);
        let resp = self
            .client
            .post(&url)
            .form(form)
            .send()
            .await
            .map_err(|e| ServiceError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(ServiceError::Upstream(format!(
                "login failed with status {}",
                resp.status()
            )));
        }

        let cookie = resp
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(';').next())
            .map(|s| s.to_string())
            .ok_or_else(|| ServiceError::Upstream("no Set-Cookie in login response".into()))?;

        Ok(MinifluxSession(cookie))
    }

    /// Forward an arbitrary request to `{base_url}{path_and_query}`.
    ///
    /// `headers` and `body` are passed through verbatim (the caller is
    /// responsible for stripping hop-by-hop headers). Returns the raw
    /// `reqwest::Response` so the caller can inspect status, headers, and
    /// body.
    pub async fn forward(
        &self,
        method: reqwest::Method,
        path_and_query: &str,
        headers: reqwest::header::HeaderMap,
        body: bytes::Bytes,
    ) -> SvcResult<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path_and_query);
        let mut req = self.client.request(method, &url).headers(headers);
        if !body.is_empty() {
            req = req.body(body);
        }
        req.send()
            .await
            .map_err(|e| ServiceError::Network(e.to_string()))
    }
}
