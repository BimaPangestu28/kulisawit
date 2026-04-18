//! SQLite connection pool and migration runner.

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::str::FromStr;

use crate::error::DbResult;

pub type DbPool = Pool<Sqlite>;

/// Open (or create) a SQLite database at the given URL or path.
///
/// Accepts `sqlite::memory:`, `sqlite://path?...`, or a bare filesystem path.
pub async fn connect(url_or_path: &str) -> DbResult<DbPool> {
    let opts = if url_or_path.starts_with("sqlite:") {
        SqliteConnectOptions::from_str(url_or_path)?
    } else {
        SqliteConnectOptions::new()
            .filename(url_or_path)
            .create_if_missing(true)
    }
    .journal_mode(SqliteJournalMode::Wal)
    .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

/// Apply all pending migrations from the `migrations/` directory at the
/// workspace root.
pub async fn migrate(pool: &DbPool) -> DbResult<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}
