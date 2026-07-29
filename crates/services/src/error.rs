//! Scaffold only — real thiserror impl added in PR#5.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("placeholder")]
    Placeholder,
}

pub type Result<T, E = ServiceError> = std::result::Result<T, E>;
