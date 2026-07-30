//! Tower catch-all Miniflux proxy layer.
//!
//! Forwards unmatched requests to a Miniflux upstream via
//! [`MinifluxClient::forward`]. For 200 `text/html` responses the proxy
//! post-processes the body:
//!
//! 1. **Strip CSP** — remove the `Content-Security-Policy` response header
//!    and any `<meta http-equiv="Content-Security-Policy" …>` tags.
//! 2. **Bilingual inject** — for elements matching `.entry-content` or
//!    `.article-content`, extract their inner text, split into sentences
//!    via [`common_text::split_sentences`], optionally translate via a
//!    [`TranslateService`], render via
//!    [`common_text::render_bilingual_div`], and replace the element's
//!    inner HTML.
//! 3. **Script injection** — append `<script src="/_inject_rs.js"></script>`
//!    before `</body>`.
//!
//! Non-200 or non-`text/html` responses are passed through unchanged.
//! Network errors map to 502 Bad Gateway via [`AppHttpError`].

use std::sync::Arc;

use axum::response::Response;
use http::StatusCode;
use reqwest::header::HeaderMap;
use scraper::{Html, Selector};
use services::{MinifluxClient, TranslateService};

use common_text::{render_bilingual_div, split_sentences, BilingualSentence, LanguageHint};

use crate::error::AppHttpError;

/// The `/_inject_rs.js` script tag appended before `</body>`.
const INJECT_SCRIPT_TAG: &str = r#"<script src="/_inject_rs.js"></script>"#;

/// Catch-all proxy layer that forwards unmatched requests to a Miniflux
/// upstream and post-processes HTML responses.
///
/// All fields are `Arc`-backed, so cloning is cheap and the struct is
/// `Send + Sync`. The layer is intended to be invoked from an Axum
/// fallback handler.
#[derive(Clone)]
pub struct MinifluxProxyLayer {
    /// Miniflux HTTP client used to forward requests.
    pub miniflux: Arc<MinifluxClient>,
    /// Optional translate service. When `Some`, sentence batches extracted
    /// from `.entry-content` / `.article-content` elements are translated
    /// before bilingual rendering.
    pub translate: Option<Arc<dyn TranslateService>>,
}

impl MinifluxProxyLayer {
    /// Construct a new proxy layer targeting `miniflux` with an optional
    /// translate service.
    pub fn new(
        miniflux: Arc<MinifluxClient>,
        translate: Option<Arc<dyn TranslateService>>,
    ) -> Self {
        Self {
            miniflux,
            translate,
        }
    }

    /// Forward `method`/`path_and_query`/`headers`/`body` to the Miniflux
    /// upstream and return a post-processed [`Response`].
    ///
    /// On network failure, returns [`AppHttpError::Service`] which maps to
    /// a 502 Bad Gateway response via `IntoResponse`.
    pub async fn proxy(
        &self,
        method: reqwest::Method,
        path_and_query: &str,
        headers: HeaderMap,
        body: bytes::Bytes,
    ) -> Result<Response, AppHttpError> {
        let upstream = self
            .miniflux
            .forward(method, path_and_query, headers, body)
            .await?;

        let status = upstream.status();
        let content_type = upstream
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        // Copy upstream headers, stripping Content-Security-Policy.
        let mut out_headers = HeaderMap::new();
        for (k, v) in upstream.headers().iter() {
            if k == "content-security-policy" {
                continue;
            }
            out_headers.insert(k.clone(), v.clone());
        }

        let body_bytes = upstream
            .bytes()
            .await
            .map_err(|e| AppHttpError::Internal(format!("upstream body read failed: {e}")))?;

        // Only post-process HTML 200 responses.
        let is_html_ok = status == StatusCode::OK && content_type.contains("text/html");
        if !is_html_ok {
            return Ok(build_response(status, out_headers, body_bytes));
        }

        let html_str = String::from_utf8_lossy(&body_bytes).into_owned();
        let processed = self.process_html(&html_str).await;
        // Body length changed → drop any stale Content-Length so the
        // framework recomputes it.
        out_headers.remove("content-length");
        Ok(build_response(status, out_headers, processed.into()))
    }

    /// Run the full HTML post-processing pipeline on `html`:
    /// strip CSP meta → inject bilingual → inject script tag.
    async fn process_html(&self, html: &str) -> String {
        let stripped = strip_csp_meta_tags(html);
        let injected = self.inject_bilingual(&stripped).await;
        inject_script_tag(&injected)
    }

    /// For each `.entry-content` / `.article-content` element, extract the
    /// inner text, split into sentences, optionally translate, render the
    /// bilingual HTML, and replace the element's inner HTML in `html`.
    ///
    /// Elements with empty text or with no translate service configured
    /// are skipped (the latter leaves the original content in place).
    async fn inject_bilingual(&self, html: &str) -> String {
        let Some(translate) = &self.translate else {
            return html.to_string();
        };

        // Parse and collect element data in a sync block so the !Send
        // scraper::Html is dropped before any .await.
        let elements: Vec<(String, String, Vec<String>)> = {
            let document = Html::parse_document(html);
            let Ok(selector) = Selector::parse(".entry-content, .article-content") else {
                return html.to_string();
            };
            document
                .select(&selector)
                .map(|el| {
                    let outer = el.html();
                    let inner = el.inner_html();
                    let text: String = el.text().collect::<Vec<_>>().join("");
                    let sentences = split_sentences(&text, LanguageHint::Detect);
                    (outer, inner, sentences)
                })
                .collect()
        };
        // document is now dropped — safe to .await

        let mut result = html.to_string();
        for (outer, inner, sentences) in elements {
            if sentences.is_empty() {
                continue;
            }
            let zh = match translate.translate_batch(sentences.clone()).await {
                Ok(zh) => zh,
                Err(_) => continue,
            };
            let bilingual = pair_bilingual(sentences, zh);
            if bilingual.is_empty() {
                continue;
            }
            let rendered = render_bilingual_div(&bilingual);
            // Rebuild outer HTML: keep opening + closing tags, swap inner.
            let new_outer = outer.replacen(&inner, &rendered, 1);
            if new_outer == outer {
                // Inner substring not found inside outer (scraper
                // normalisation mismatch) — leave the element unchanged.
                continue;
            }
            result = result.replacen(&outer, &new_outer, 1);
        }
        result
    }
}

/// Pair `src` and `zh` vectors into [`BilingualSentence`]s with sequential
/// byte offsets into the concatenated source text.
fn pair_bilingual(src: Vec<String>, zh: Vec<String>) -> Vec<BilingualSentence> {
    let mut out = Vec::with_capacity(src.len().min(zh.len()));
    let mut offset = 0usize;
    for (s, z) in src.into_iter().zip(zh.into_iter()) {
        let start = offset;
        offset += s.len();
        let end = offset;
        out.push(BilingualSentence {
            src: s,
            zh: z,
            src_start: start,
            src_end: end,
        });
    }
    out
}

/// Strip every `<meta http-equiv="Content-Security-Policy" …>` tag from
/// `html` using a case-insensitive byte-level scan. Non-CSP `<meta>` tags
/// are preserved verbatim.
fn strip_csp_meta_tags(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut result = String::with_capacity(html.len());
    let mut last_end = 0usize;
    let mut search_from = 0usize;
    while let Some(rel_start) = lower[search_from..].find("<meta") {
        let abs_start = search_from + rel_start;
        let Some(rel_end) = lower[abs_start..].find('>') else {
            break;
        };
        let abs_end = abs_start + rel_end + 1;
        let tag = &lower[abs_start..abs_end];
        if tag.contains("http-equiv") && tag.contains("content-security-policy") {
            result.push_str(&html[last_end..abs_start]);
            last_end = abs_end;
        }
        search_from = abs_end;
    }
    result.push_str(&html[last_end..]);
    result
}

/// Insert `<script src="/_inject_rs.js"></script>` immediately before the
/// first `</body>` tag. If no `</body>` is present, append the script at
/// the end of `html`.
fn inject_script_tag(html: &str) -> String {
    let Some(pos) = html.find("</body>") else {
        let mut s = String::with_capacity(html.len() + INJECT_SCRIPT_TAG.len());
        s.push_str(html);
        s.push_str(INJECT_SCRIPT_TAG);
        return s;
    };
    let mut s = String::with_capacity(html.len() + INJECT_SCRIPT_TAG.len());
    s.push_str(&html[..pos]);
    s.push_str(INJECT_SCRIPT_TAG);
    s.push_str(&html[pos..]);
    s
}

/// Assemble an [`axum::response::Response`] from `status`, `headers`, and
/// `body` bytes.
fn build_response(status: StatusCode, headers: HeaderMap, body: bytes::Bytes) -> Response {
    let mut resp: Response = http::Response::new(axum::body::Body::from(body));
    *resp.status_mut() = status;
    for (k, v) in headers.iter() {
        resp.headers_mut().insert(k.clone(), v.clone());
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_csp_meta_removes_only_csp_metas_case_insensitive() {
        let html = "<html><head>\
            <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'\">\
            <meta name=\"description\" content=\"keep me\">\
            <META HTTP-EQUIV=\"content-security-policy\" CONTENT=\"x\">\
            </head></html>";
        let out = strip_csp_meta_tags(html);
        assert!(!out.to_ascii_lowercase().contains("content-security-policy"));
        assert!(out.contains("keep me"));
    }

    #[test]
    fn inject_script_before_body_close_or_appends() {
        let with_body = "<html><body>x</body></html>";
        let out = inject_script_tag(with_body);
        let script_idx = out.find(INJECT_SCRIPT_TAG).unwrap();
        let body_idx = out.find("</body>").unwrap();
        assert!(script_idx < body_idx);

        let no_body = "<html>no body close";
        let out = inject_script_tag(no_body);
        assert!(out.ends_with(INJECT_SCRIPT_TAG));
    }

    #[test]
    fn pair_bilingual_assigns_sequential_offsets() {
        let src = vec!["Hello.".to_string(), "World.".to_string()];
        let zh = vec!["你好。".to_string(), "世界。".to_string()];
        let pairs = pair_bilingual(src, zh);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].src_start, 0);
        assert_eq!(pairs[0].src_end, 6);
        assert_eq!(pairs[1].src_start, 6);
        assert_eq!(pairs[1].src_end, 12);
    }

    #[test]
    fn build_response_preserves_status_headers_and_body() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test", "v".parse().unwrap());
        let body = bytes::Bytes::from_static(b"hi");
        let resp = build_response(StatusCode::OK, headers, body);
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.headers().get("x-test").unwrap(), "v");
    }
}
