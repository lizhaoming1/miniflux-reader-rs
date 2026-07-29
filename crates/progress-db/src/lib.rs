//! Scaffold only — real sqlx models/repos in PR#4.

pub mod error {
    use thiserror::Error;
    #[derive(Debug, Error)] pub enum DbError { #[error("placeholder")] Placeholder }
    pub type Result<T, E = DbError> = std::result::Result<T, E>;
}
pub use error::{DbError, Result as DbResult};
pub mod models {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct ReadingProgress {
        pub epub_path:   String,
        pub chapter_idx: i32,
        pub scroll_pos:  i32,
        pub percent:     f32,
        pub overall:     f32,
    }
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct Book {
        pub safe_name: String,
        pub title:     String,
    }
}
pub mod repository {
    use super::*;
    pub struct ProgressRepository;
    impl ProgressRepository { pub fn placeholder() -> Self { Self } }
    pub struct BookRepository;
    impl BookRepository { pub fn placeholder() -> Self { Self } }
}
pub mod migrate {
    use super::DbResult;
    pub async fn run_migrations(_db: ()) -> DbResult<()> { Ok(()) }
}
