//! Full-article extraction via readability for feeds that only ship
//! summaries.

use std::io::Cursor;
use std::time::Duration;

use crate::{FeedEngineError, Result};

/// Download `url` and run readability extraction to get sanitized HTML
/// of the article body.
pub async fn extract_content(url: &str, timeout: Duration) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| FeedEngineError::Network(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| FeedEngineError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(FeedEngineError::UpstreamStatus(resp.status().as_u16()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| FeedEngineError::Network(e.to_string()))?;
    // readability::extractor::extract takes `&mut R: Read` + `&url::Url`.
    // The reqwest workspace dep re-exports url::Url as reqwest::Url.
    let parsed_url = reqwest::Url::parse(url)
        .map_err(|e| FeedEngineError::Network(format!("invalid url: {e}")))?;
    let mut cursor = Cursor::new(bytes.to_vec());
    let doc = readability::extractor::extract(&mut cursor, &parsed_url)
        .map_err(|e| FeedEngineError::Parse(e.to_string()))?;
    Ok(doc.content)
}
