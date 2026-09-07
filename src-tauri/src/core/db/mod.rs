pub mod error;
pub mod models;
pub mod store;

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

pub async fn init(data_dir: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(data_dir.join("app.db"))
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!("src/core/db/migrations").run(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(":memory:")
                .foreign_keys(true),
        )
        .await
        .expect("in-memory pool");
    sqlx::migrate!("src/core/db/migrations")
        .run(&pool)
        .await
        .expect("migrations");
    pool
}

#[cfg(test)]
mod tests {
    /// sqlx checksums a shipped migration, so a step is only ever a new file.
    #[test]
    fn migrations_are_append_only() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core/db/migrations");
        let mut numbers = vec![];
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let stem = name
                .strip_suffix(".sql")
                .unwrap_or_else(|| panic!("not a migration: {name}"));
            let (prefix, rest) = stem.split_at_checked(4).unwrap_or(("", ""));
            assert!(
                prefix.chars().all(|c| c.is_ascii_digit()) && rest.starts_with('_'),
                "a migration is named 00NN_what_it_does.sql: {name}"
            );
            numbers.push(prefix.parse::<u32>().unwrap());
        }
        numbers.sort_unstable();
        let expected: Vec<u32> = (1..=numbers.len() as u32).collect();
        assert_eq!(
            numbers, expected,
            "migration numbers run 0001..N with no gap and no repeat"
        );
    }
}
