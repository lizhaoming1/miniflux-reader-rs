//! Unified HTTP error type that maps service / db / epub errors into
//! structured JSON responses with a `req_id` correlation ID.

use axum::response::{IntoResponse, Json};
use http::StatusCode;
use serde_json::json;

use epub_lib::EpubError;
use progress_db::DbError;
use services::ServiceError;

/// All errors that can arise while handling an HTTP request.
#[derive(Debug)]
pub enum AppHttpError {
    /// A service-layer error (translate / TTS / miniflux).
    Service(ServiceError),
    /// A database-layer error.
    Db(DbError),
    /// An EPUB-layer error.
    Epub(EpubError),
    /// The request was malformed (bad JSON, missing field, …).
    BadRequest(String),
    /// The requested resource was not found.
    NotFound(String),
    /// An unexpected internal error.
    Internal(String),
}

impl From<ServiceError> for AppHttpError {
    fn from(e: ServiceError) -> Self {
        AppHttpError::Service(e)
    }
}

impl From<DbError> for AppHttpError {
    fn from(e: DbError) -> Self {
        AppHttpError::Db(e)
    }
}

impl From<EpubError> for AppHttpError {
    fn from(e: EpubError) -> Self {
        AppHttpError::Epub(e)
    }
}

impl IntoResponse for AppHttpError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, msg) = match &self {
            AppHttpError::Service(ServiceError::TextTooLong(n)) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "TEXT_TOO_LONG",
                format!("batch too long: {n}"),
            ),
            AppHttpError::Service(ServiceError::Timeout(d)) => (
                StatusCode::GATEWAY_TIMEOUT,
                "TIMEOUT",
                format!("timed out: {d:?}"),
            ),
            AppHttpError::Service(ServiceError::Network(m)) => {
                (StatusCode::BAD_GATEWAY, "BAD_GATEWAY", m.clone())
            }
            AppHttpError::Service(ServiceError::Upstream(m)) => {
                (StatusCode::BAD_GATEWAY, "UPSTREAM", m.clone())
            }
            AppHttpError::Db(DbError::NotFound(k)) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("not found: {k}"),
            ),
            AppHttpError::Db(e) => (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", e.to_string()),
            AppHttpError::Epub(e) => (StatusCode::BAD_REQUEST, "EPUB_ERROR", e.to_string()),
            AppHttpError::BadRequest(m) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", m.clone()),
            AppHttpError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m.clone()),
            AppHttpError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", m.clone()),
        };
        let req_id = uuid::Uuid::new_v4().simple().to_string();
        (
            status,
            Json(json!({ "code": code, "msg": msg, "req_id": req_id })),
        )
            .into_response()
    }
}
