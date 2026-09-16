//! One way out of the process: state on disk, in SQLite.

mod error;

use std::path::Path;
use std::time::Duration;

pub use error::{Error, Result};
pub use sqlx;
pub use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The database: a pool over one file, migrated on open.
#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Opens or creates `path` and brings its schema up to date.
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(BUSY_TIMEOUT)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        Self::migrated(pool).await
    }

    /// A private in-memory database with the full schema, for tests. One connection:
    /// every query sees the same memory.
    pub async fn in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        Self::migrated(pool).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    async fn migrated(pool: SqlitePool) -> Result<Self> {
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests;
