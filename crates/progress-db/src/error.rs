//! Error types for the progress-db layer.

use thiserror::Error;

/// All database-layer errors.
#[derive(Debug, Error)]
pub enum DbError {
    /// sqlx returned an error (query failure, pool issue, …).
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// sqlx migrate reported an error.
    #[error("migrate error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// A requested row was not found (e.g. `get_progress` for an unknown
    /// `epub_path`).
    #[error("not found: {0}")]
    NotFound(String),
}

/// Convenience `Result` alias.
pub type Result<T, E = DbError> = std::result::Result<T, E>;
