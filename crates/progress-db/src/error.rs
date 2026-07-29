//! Scaffold only — real thiserror impl added in PR#4.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("placeholder")]
    Placeholder,
}

pub type Result<T, E = DbError> = std::result::Result<T, E>;
