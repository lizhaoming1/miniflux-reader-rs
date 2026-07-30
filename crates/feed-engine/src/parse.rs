//! Parsing of RSS / Atom feeds via `feed-rs`.

use std::time::Duration;

use chrono::{DateTime, Utc};
use feed_rs::parser;
use serde::{Deserialize, Serialize};

use crate::{FeedEngineError, Result};

/// Output of a successful feed parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFeed {
    /// Feed title from `<channel><title>` or Atom `<title>`.
    pub title: String,
    /// Site URL parsed from feed `<link>`.
    pub site_url: String,
    /// Individual articles / entries found in this feed.
    pub articles: Vec<ParsedArticle>,
}

/// A single parsed feed entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedArticle {
    /// Feed-scoped unique identifier (RSS `<guid>` or Atom `<id>`).
    pub guid: String,
    /// Entry title.
    pub title: String,
    /// Original article URL (RSS `<link>` or Atom alternate link).
    pub url: String,
    /// Author string if available.
    pub author: Option<String>,
    /// Short summary text / description.
    pub summary: String,
    /// Full HTML body if the feed exposes it.
    pub content_html: Option<String>,
    /// Publication timestamp normalized to UTC.
    pub published_at: DateTime<Utc>,
}

/// Fetch a feed over HTTP and parse it into a [`ParsedFeed`].
///
/// `timeout` bounds the HTTP request; a `user_agent` string (if set) is
/// sent as the User-Agent header.
pub async fn fetch_feed(
    url: &str,
    timeout: Duration,
    user_agent: Option<&str>,
) -> Result<ParsedFeed> {
    let client = build_client(timeout)?;
    let mut req = client.get(url);
    if let Some(ua) = user_agent {
        req = req.header(reqwest::header::USER_AGENT, ua);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| FeedEngineError::Network(e.to_string()))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(FeedEngineError::UpstreamStatus(status.as_u16()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| FeedEngineError::Network(e.to_string()))?;
    parse_feed_bytes(&bytes)
}

fn build_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| FeedEngineError::Network(e.to_string()))
}

/// Parse raw feed bytes (RSS or Atom) into a [`ParsedFeed`].
///
/// Public so integration tests can exercise the parser without HTTP.
pub fn parse_feed_bytes(bytes: &[u8]) -> Result<ParsedFeed> {
    let feed = parser::parse(bytes).map_err(|e| FeedEngineError::Parse(e.to_string()))?;
    let title = feed.title.map(|t| t.content).unwrap_or_default();
    let site_url = feed
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("alternate") || l.rel.is_none())
        .map(|l| l.href.clone())
        .unwrap_or_default();

    let mut articles = Vec::with_capacity(feed.entries.len());
    for entry in feed.entries {
        let guid = entry.id.clone();
        if guid.is_empty() {
            continue; // skip entries without stable id — can't dedupe
        }
        let title = entry.title.map(|t| t.content).unwrap_or_default();
        let url = entry
            .links
            .iter()
            .find(|l| l.rel.as_deref() == Some("alternate") || l.rel.is_none())
            .map(|l| l.href.clone())
            .unwrap_or_default();
        let author = entry.authors.first().map(|a| a.name.clone());
        let summary = entry
            .summary
            .as_ref()
            .map(|s| s.content.clone())
            .unwrap_or_default();
        let content_html = entry.content.as_ref().and_then(|c| c.body.clone());
        let published_at = entry
            .published
            .or(entry.updated)
            .unwrap_or_else(Utc::now);
        articles.push(ParsedArticle {
            guid,
            title,
            url,
            author,
            summary,
            content_html,
            published_at,
        });
    }
    Ok(ParsedFeed {
        title,
        site_url,
        articles,
    })
}
