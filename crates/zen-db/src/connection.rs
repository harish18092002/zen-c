use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

/// Connect to a SQLite database file at `path`. Creates the file (and any
/// missing parent directories) if it does not exist. Configures sane defaults
/// for an embedded desktop app: WAL journaling, NORMAL synchronous, FK on,
/// 30-second busy timeout to ride out short contention windows.
pub async fn connect_file(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
    }

    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(30));

    SqlitePoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(opts)
        .await
}

/// Open an in-memory database with FK on. Each call gets a fresh database
/// because `:memory:` is per-connection — we cap the pool to 1 so subsequent
/// queries see the same data.
pub async fn connect_memory() -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:")?
        .create_if_missing(true)
        .foreign_keys(true);

    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
}

/// Legacy URL-based connect kept for callers that already have a URL string.
/// Prefer `connect_file` / `connect_memory` for new code.
pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(30));

    SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
}

/// Apply runtime PRAGMAs. WAL/synchronous/foreign_keys are also set via
/// `SqliteConnectOptions` above, but applying them at the pool level too is a
/// no-op safety net if a caller uses `connect()` with an unconfigured URL.
pub async fn configure(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA journal_mode=WAL").execute(pool).await?;
    sqlx::query("PRAGMA foreign_keys=ON").execute(pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL")
        .execute(pool)
        .await?;
    Ok(())
}
