use sqlx::SqlitePool;
use std::path::Path;

pub mod load;
pub mod schema;

pub async fn init(data_dir: &Path) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(data_dir.join("app.db"))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            // The app manages deletes/re-points manually (no ON DELETE CASCADE),
            // e.g. explorer→task conversion re-points child rows and discard tears
            // a task down piecewise. sqlx enables foreign_keys by default, which
            // breaks those ordered operations — turn enforcement off to match the
            // intended design.
            .foreign_keys(false),
    )
    .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;

    Ok(pool)
}
