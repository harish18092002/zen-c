use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}

pub async fn configure(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA journal_mode=WAL").execute(pool).await?;
    sqlx::query("PRAGMA foreign_keys=ON").execute(pool).await?;
    sqlx::query("PRAGMA synchronous=NORMAL").execute(pool).await?;
    Ok(())
}
