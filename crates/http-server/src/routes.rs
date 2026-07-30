//! Axum route builder and handlers.
//!
//! Wires together workspace crates behind a single Axum `Router`. Every
//! handler uses the shared [`AppState`] and returns
//! `Result<impl IntoResponse, AppHttpError>`. v0.2.0 dropped the
//! catch-all Miniflux proxy layer + /mf-login route in favour of
//! explicit routes for feeds, articles, OPML, and settings.

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get, post};
use axum::Router;
use http::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

use crate::error::AppHttpError;
use crate::state::AppState;

/// Maximum accepted EPUB upload size (50 MiB).
const EPUB_UPLOAD_LIMIT: usize = 50 * 1024 * 1024;

/// Default list_all article page size when the client doesn't specify.
const DEFAULT_ARTICLE_LIMIT: u32 = 50;
/// Maximum client-provided page size.
const MAX_ARTICLE_LIMIT: u32 = 200;
/// Large upper bound used when listing all unread articles for a feed
/// (avoids `u32::MAX` which can overflow downstream SQL bindings).
const UNREAD_LIST_CAP: u32 = 100_000;

/// Build the Axum router with all application routes.
pub fn build_axum_routes(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        // ---- Feeds ----
        .route("/feeds", get(list_feeds).post(add_feed))
        .route("/feeds/:id", delete(remove_feed))
        .route("/feeds/sync", post(sync_all_handler))
        // ---- Articles ----
        .route("/articles", get(list_articles))
        .route("/articles/:id", get(get_article))
        .route("/articles/:id/read", post(set_article_read))
        .route("/articles/read-all", post(mark_all_read))
        .route("/articles/unread-count", get(unread_count))
        // ---- OPML ----
        .route("/opml/import", post(opml_import))
        .route("/opml/export", get(opml_export))
        // ---- Settings ----
        .route("/settings", get(get_settings).put(update_settings))
        // ---- EPUB ----
        .route(
            "/epub/upload",
            post(upload_epub).layer(axum::extract::DefaultBodyLimit::max(EPUB_UPLOAD_LIMIT)),
        )
        .route(
            "/epub/api/progress/:safe_name",
            get(get_progress).post(save_progress),
        )
        // ---- Translate / TTS ----
        .route("/translate", get(translate))
        .route("/tts", get(tts))
        .route("/tts_highlight", post(tts_highlight))
        .with_state(state)
}

// =====================================================================
// Preserved handlers (healthz, EPUB, translate, TTS)
// =====================================================================

async fn healthz(_state: State<AppState>) -> Result<impl IntoResponse, AppHttpError> {
    let req_id = uuid::Uuid::new_v4().simple().to_string();
    Ok(Json(json!({ "ok": true, "req_id": req_id })))
}

async fn upload_epub(
    State(_state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppHttpError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppHttpError::BadRequest(e.to_string()))?
    {
        if field.name() == Some("file") {
            let filename = field.file_name().unwrap_or("unnamed").to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| AppHttpError::BadRequest(e.to_string()))?;
            if bytes.len() > EPUB_UPLOAD_LIMIT {
                return Err(AppHttpError::BadRequest("file too large".into()));
            }
            let safe_name = epub_lib::save_upload_to_disk(&bytes, &filename)?;
            return Ok(Json(json!({ "ok": true, "safe_name": safe_name })));
        }
    }
    Err(AppHttpError::BadRequest("missing file field".into()))
}

async fn get_progress(
    State(state): State<AppState>,
    Path(safe_name): Path<String>,
) -> Result<impl IntoResponse, AppHttpError> {
    let repo = progress_db::ProgressRepository::new(state.db);
    let progress = repo.get(&safe_name).await?;
    Ok(Json(progress))
}

async fn save_progress(
    State(state): State<AppState>,
    Path(safe_name): Path<String>,
    Json(mut p): Json<progress_db::ReadingProgress>,
) -> Result<impl IntoResponse, AppHttpError> {
    p.epub_path = safe_name;
    let repo = progress_db::ProgressRepository::new(state.db);
    repo.save(&p).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn translate(
    State(state): State<AppState>,
    Query(q): Query<TextQuery>,
) -> Result<impl IntoResponse, AppHttpError> {
    let translations = state.translate.translate_batch(vec![q.text]).await?;
    Ok(translations.into_iter().next().unwrap_or_default())
}

async fn tts(
    State(state): State<AppState>,
    Query(q): Query<TextQuery>,
) -> Result<impl IntoResponse, AppHttpError> {
    let bytes = state.tts.synthesize(&q.text).await?;
    Ok(([(header::CONTENT_TYPE, "audio/mpeg")], bytes))
}

async fn tts_highlight(
    State(state): State<AppState>,
    Json(req): Json<HighlightRequest>,
) -> Result<impl IntoResponse, AppHttpError> {
    let tokens = state.tts.highlights(&req.text).await?;
    Ok(Json(tokens))
}

// =====================================================================
// Feed handlers
// =====================================================================

async fn list_feeds(State(state): State<AppState>) -> Result<impl IntoResponse, AppHttpError> {
    let feeds = state.feed_repo.list().await?;
    let mut out = Vec::with_capacity(feeds.len());
    for f in feeds {
        let unread = state
            .article_repo
            .list_unread(Some(f.id), UNREAD_LIST_CAP)
            .await?
            .len() as i64;
        out.push(json!({
            "id": f.id, "url": f.url, "title": f.title,
            "site_url": f.site_url, "last_fetched": f.last_fetched,
            "fetch_error": f.fetch_error, "unread_count": unread,
        }));
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
struct AddFeedRequest {
    url: String,
}

async fn add_feed(
    State(state): State<AppState>,
    Json(body): Json<AddFeedRequest>,
) -> Result<impl IntoResponse, AppHttpError> {
    if body.url.is_empty() {
        return Err(AppHttpError::BadRequest("url is required".into()));
    }
    let feed = state.feed_repo.add(&body.url).await?;
    // Trigger an immediate background sync so the user sees articles right away.
    let pool = state.db.clone();
    let fid = feed.id;
    let ua = state.settings_repo.get("feed.user_agent").await?;
    let timeout_secs: u64 = state
        .settings_repo
        .get("feed.fetch_timeout_secs")
        .await?
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    tokio::spawn(async move {
        let cfg = feed_engine::sync::SyncConfig {
            fetch_timeout: Duration::from_secs(timeout_secs),
            user_agent: ua,
        };
        match feed_engine::sync::sync_feed(&pool, fid, &cfg).await {
            Ok(n) => tracing::info!(feed_id = fid, new = n, "on-demand sync completed"),
            Err(e) => tracing::warn!(feed_id = fid, err = %e, "on-demand sync failed"),
        }
    });
    Ok((
        StatusCode::CREATED,
        Json(json!({ "ok": true, "id": feed.id, "title": feed.title })),
    ))
}

async fn remove_feed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppHttpError> {
    state.feed_repo.remove(id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn sync_all_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppHttpError> {
    let ua = state.settings_repo.get("feed.user_agent").await?;
    let timeout_secs: u64 = state
        .settings_repo
        .get("feed.fetch_timeout_secs")
        .await?
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    let sync_cfg = feed_engine::sync::SyncConfig {
        fetch_timeout: Duration::from_secs(timeout_secs),
        user_agent: ua,
    };
    // Run sync in the background for large feed counts; return immediately.
    let pool = state.db.clone();
    tokio::spawn(async move {
        let n = feed_engine::sync::sync_all_feeds(&pool, &sync_cfg).await;
        tracing::info!(new = n, "manual sync_all finished");
    });
    Ok(Json(json!({ "ok": true, "message": "sync started in background" })))
}

// =====================================================================
// Article handlers
// =====================================================================

#[derive(Deserialize)]
struct ArticlesQuery {
    feed_id: Option<i64>,
    unread_only: Option<bool>,
    limit: Option<u32>,
    offset: Option<u32>,
}

async fn list_articles(
    State(state): State<AppState>,
    Query(q): Query<ArticlesQuery>,
) -> Result<impl IntoResponse, AppHttpError> {
    let limit = q.limit.unwrap_or(DEFAULT_ARTICLE_LIMIT).min(MAX_ARTICLE_LIMIT);
    let offset = q.offset.unwrap_or(0);
    let articles = if q.unread_only.unwrap_or(false) {
        state.article_repo.list_unread(q.feed_id, limit).await?
    } else {
        state.article_repo.list_all(q.feed_id, limit, offset).await?
    };
    let feed_title_map: HashMap<i64, String> = state
        .feed_repo
        .list()
        .await?
        .into_iter()
        .map(|f| (f.id, f.title))
        .collect();
    let out: Vec<_> = articles
        .into_iter()
        .map(|a| {
            json!({
                "id": a.id,
                "feed_id": a.feed_id,
                "feed_title": feed_title_map.get(&a.feed_id).cloned().unwrap_or_default(),
                "title": a.title,
                "url": a.url,
                "author": a.author,
                "summary": a.summary,
                "published_at": a.published_at,
                "read": a.is_read,
            })
        })
        .collect();
    Ok(Json(out))
}

async fn get_article(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppHttpError> {
    let mut a = state.article_repo.get(id).await?;
    // If article has no content_html, try readability extraction once.
    let html_parts = if let Some(ref body) = a.content_html {
        body.clone()
    } else {
        let timeout_secs: u64 = state
            .settings_repo
            .get("feed.fetch_timeout_secs")
            .await?
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        match feed_engine::content::extract_content(&a.url, Duration::from_secs(timeout_secs))
            .await
        {
            Ok(extracted) => {
                // Persist to DB so future reads don't re-fetch.
                let ups = progress_db::ArticleUpsert {
                    guid: a.guid.clone(),
                    title: a.title.clone(),
                    url: a.url.clone(),
                    author: a.author.clone(),
                    summary: a.summary.clone(),
                    content_html: Some(extracted.clone()),
                    published_at: a.published_at.clone(),
                };
                let _ = state.article_repo.upsert(a.feed_id, &ups).await;
                extracted
            }
            Err(e) => {
                tracing::warn!(
                    article_id = a.id,
                    err = %e,
                    "extract_content failed, falling back to summary"
                );
                a.summary.clone()
            }
        }
    };
    // Sentence-split and optionally translate.
    use common_text::{
        render_bilingual_div, split_sentences, BilingualSentence, LanguageHint,
    };
    let sentences = split_sentences(&html_parts, LanguageHint::Detect);
    // TODO(perf): translate batch via state.translate. Placeholder below
    // renders untranslated bilingual with empty zh — the
    // common_text::render_bilingual_div still produces valid markup (zh
    // paragraphs hidden via CSS if empty).
    let bilingual_sents: Vec<BilingualSentence> = sentences
        .into_iter()
        .map(|s| BilingualSentence {
            src: s,
            zh: String::new(),
            src_start: 0,
            src_end: 0,
        })
        .collect();
    // Mark article read automatically upon view.
    let _ = state.article_repo.set_read(a.id, true).await;
    a.is_read = true;
    let rendered = render_bilingual_div(&bilingual_sents);
    let body = format!(
        r#"<article data-article-id="{}"><header><h1>{}</h1><p class="meta">{}</p></header>{}</article>"#,
        a.id, a.title, a.published_at, rendered,
    );
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(axum::body::Body::from(body))
        .map_err(|e| AppHttpError::Internal(e.to_string()))?;
    Ok(response)
}

#[derive(Deserialize)]
struct SetReadRequest {
    read: bool,
}

async fn set_article_read(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<SetReadRequest>,
) -> Result<impl IntoResponse, AppHttpError> {
    state.article_repo.set_read(id, body.read).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct MarkAllQuery {
    feed_id: Option<i64>,
}

async fn mark_all_read(
    State(state): State<AppState>,
    Query(q): Query<MarkAllQuery>,
) -> Result<impl IntoResponse, AppHttpError> {
    state.article_repo.mark_all_read(q.feed_id).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn unread_count(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppHttpError> {
    let c = state.article_repo.count_unread().await?;
    Ok(Json(json!({ "count": c })))
}

// =====================================================================
// OPML handlers
// =====================================================================

async fn opml_import(
    State(state): State<AppState>,
    body: String,
) -> Result<impl IntoResponse, AppHttpError> {
    let urls = feed_engine::opml::parse_opml(&body);
    let mut added = 0;
    let mut failed = 0;
    for u in urls {
        match state.feed_repo.add(&u).await {
            Ok(_) => added += 1,
            Err(_) => failed += 1,
        }
    }
    // Trigger one sync pass for new feeds in background.
    let pool = state.db.clone();
    let ua = state.settings_repo.get("feed.user_agent").await?;
    let timeout_secs: u64 = state
        .settings_repo
        .get("feed.fetch_timeout_secs")
        .await?
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    tokio::spawn(async move {
        let cfg = feed_engine::sync::SyncConfig {
            fetch_timeout: Duration::from_secs(timeout_secs),
            user_agent: ua,
        };
        let n = feed_engine::sync::sync_all_feeds(&pool, &cfg).await;
        tracing::info!(new = n, "post-OPML sync_all finished");
    });
    Ok(Json(json!({ "ok": true, "added": added, "failed": failed })))
}

async fn opml_export(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppHttpError> {
    let feeds = state.feed_repo.list().await?;
    let xml = feed_engine::opml::export_opml(&feeds);
    Ok((
        [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
        xml,
    ))
}

// =====================================================================
// Settings handlers
// =====================================================================

/// Runtime-overridable keys. Anything else is rejected to prevent the
/// table from becoming an arbitrary storage bucket.
const ALLOWED_SETTINGS_KEYS: &[&str] = &[
    "feed.poll_interval_secs",
    "feed.fetch_timeout_secs",
    "feed.user_agent",
    "translate.target_lang",
    "tts.voice",
];

#[derive(Serialize)]
struct SettingsValue {
    key: String,
    value: String,
}

async fn get_settings(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppHttpError> {
    let stored = state.settings_repo.get_all().await?;
    let out: Vec<SettingsValue> = ALLOWED_SETTINGS_KEYS
        .iter()
        .map(|k| {
            let val = stored.get(*k).cloned().unwrap_or_default();
            SettingsValue {
                key: (*k).to_string(),
                value: val,
            }
        })
        .collect();
    Ok(Json(out))
}

async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<HashMap<String, String>>,
) -> Result<impl IntoResponse, AppHttpError> {
    // Validate keys.
    for k in body.keys() {
        if !ALLOWED_SETTINGS_KEYS.contains(&k.as_str()) {
            return Err(AppHttpError::BadRequest(format!("unknown setting key: {k}")));
        }
    }
    state.settings_repo.set_many(&body).await?;
    Ok(Json(json!({ "ok": true, "updated": body.len() })))
}

// =====================================================================
// Shared request types (private)
// =====================================================================

#[derive(Deserialize)]
struct TextQuery {
    text: String,
}

#[derive(Deserialize)]
struct HighlightRequest {
    text: String,
}
