//! Axum route builder and handlers.
//!
//! Wires together all workspace crates (`epub-lib`, `progress-db`,
//! `services`) behind a single Axum `Router`. Every handler uses the
//! shared [`AppState`] and returns `Result<impl IntoResponse, AppHttpError>`.

use axum::extract::{Multipart, Path, Query, State};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use http::header;
use http::{HeaderMap, Method, Uri};
use serde::Deserialize;
use serde_json::json;

use crate::error::AppHttpError;
use crate::proxy_layer::MinifluxProxyLayer;
use crate::state::AppState;

/// Maximum accepted EPUB upload size (50 MiB).
const EPUB_UPLOAD_LIMIT: usize = 50 * 1024 * 1024;

/// Build the Axum router with all application routes.
pub fn build_axum_routes(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/mf-login", post(mf_login))
        .route(
            "/epub/upload",
            post(upload_epub).layer(axum::extract::DefaultBodyLimit::max(EPUB_UPLOAD_LIMIT)),
        )
        .route(
            "/epub/api/progress/:safe_name",
            get(get_progress).post(save_progress),
        )
        .route("/translate", get(translate))
        .route("/tts", get(tts))
        .route("/tts_highlight", post(tts_highlight))
        .fallback(proxy_fallback)
        .with_state(state)
}

// ---------- handlers ----------

/// `GET /healthz` → 200 `{"ok": true, "req_id": "<uuid>"}`
async fn healthz(_state: State<AppState>) -> Result<impl IntoResponse, AppHttpError> {
    let req_id = uuid::Uuid::new_v4().simple().to_string();
    Ok(Json(json!({ "ok": true, "req_id": req_id })))
}

/// `POST /mf-login` → login against Miniflux, set `MF_SESSION_RS` cookie.
async fn mf_login(
    State(state): State<AppState>,
    Json(form): Json<services::LoginForm>,
) -> Result<impl IntoResponse, AppHttpError> {
    let session = state.miniflux.login(&form).await?;
    // `MinifluxSession.0` is the raw `name=value` cookie pair from the
    // upstream; extract just the value so we can re-wrap it under our own
    // `MF_SESSION_RS` cookie name.
    let value = session.0.split('=').nth(1).unwrap_or(&session.0);
    let cookie = format!("MF_SESSION_RS={}; HttpOnly; Path=/", value);
    Ok(([(header::SET_COOKIE, cookie)], Json(json!({ "ok": true }))))
}

/// `POST /epub/upload` → accept multipart `file` field, persist to disk.
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

/// `GET /epub/api/progress/:safe_name` → return saved progress, or 404.
async fn get_progress(
    State(state): State<AppState>,
    Path(safe_name): Path<String>,
) -> Result<impl IntoResponse, AppHttpError> {
    let repo = progress_db::ProgressRepository::new(state.db);
    let progress = repo.get(&safe_name).await?;
    Ok(Json(progress))
}

/// `POST /epub/api/progress/:safe_name` → upsert progress for the path key.
async fn save_progress(
    State(state): State<AppState>,
    Path(safe_name): Path<String>,
    Json(mut p): Json<progress_db::ReadingProgress>,
) -> Result<impl IntoResponse, AppHttpError> {
    // The path param is the source of truth for the progress key.
    p.epub_path = safe_name;
    let repo = progress_db::ProgressRepository::new(state.db);
    repo.save(&p).await?;
    Ok(Json(json!({ "ok": true })))
}

/// `GET /translate?text=...` → translate the text, return first result as
/// `text/plain`.
async fn translate(
    State(state): State<AppState>,
    Query(q): Query<TextQuery>,
) -> Result<impl IntoResponse, AppHttpError> {
    let translations = state.translate.translate_batch(vec![q.text]).await?;
    Ok(translations.into_iter().next().unwrap_or_default())
}

/// `GET /tts?text=...` → synthesize audio, return bytes as `audio/mpeg`.
async fn tts(
    State(state): State<AppState>,
    Query(q): Query<TextQuery>,
) -> Result<impl IntoResponse, AppHttpError> {
    let bytes = state.tts.synthesize(&q.text).await?;
    Ok(([(header::CONTENT_TYPE, "audio/mpeg")], bytes))
}

/// `POST /tts_highlight` → `{"text":"..."}` → highlight tokens JSON array.
async fn tts_highlight(
    State(state): State<AppState>,
    Json(req): Json<HighlightRequest>,
) -> Result<impl IntoResponse, AppHttpError> {
    let tokens = state.tts.highlights(&req.text).await?;
    Ok(Json(tokens))
}

// ---------- request types ----------

#[derive(Deserialize)]
struct TextQuery {
    text: String,
}

#[derive(Deserialize)]
struct HighlightRequest {
    text: String,
}

// ---------- catch-all proxy fallback ----------

/// Fallback handler that forwards unmatched requests to the Miniflux
/// upstream via [`MinifluxProxyLayer`].
async fn proxy_fallback(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Result<axum::response::Response, AppHttpError> {
    let layer = MinifluxProxyLayer::new(state.miniflux.clone(), Some(state.translate.clone()));
    let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    layer.proxy(method, path, headers, body).await
}
