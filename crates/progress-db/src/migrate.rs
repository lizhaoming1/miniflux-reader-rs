//! Database migration runner — wraps `sqlx::migrate!()`.

use sqlx::SqlitePool;

use crate::error::Result;

/// Run all embedded migrations from the workspace `migrations/` directory.
/// Idempotent — safe to call on every startup.
pub async fn run_migrations(db: &SqlitePool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(db).await?;
    Ok(())
}
