//! The time Groove measured on a task, and how much of it the source has been told.

#[cfg(test)]
mod tests;

use groove_db::Db;
use groove_types::{Day, Error, ExternalId, Result, TimeSummary, Timestamp};

/// The ledger on disk, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Ledger {
    db: Db,
}

impl Ledger {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// A ledger on a private in-memory database, for tests up the stack.
    pub async fn in_memory() -> Result<Self> {
        let db = Db::in_memory()
            .await
            .map_err(|e| Error::invalid(format!("no database: {e}")))?;
        Ok(Self::new(db))
    }

    /// What every task the ledger holds has measured.
    pub async fn read(&self) -> Result<Vec<(ExternalId, TimeSummary)>> {
        let rows: Vec<(String, i64, i64, String, i64)> = sqlx::query_as(
            "SELECT external_id, tracked_seconds, logged_seconds, today_day, today_seconds
             FROM ledger",
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(failed)?;
        let today = Timestamp::now().day().to_string();
        Ok(rows.into_iter().map(|row| summary(row, &today)).collect())
    }

    /// Adds seconds to what a task has measured, and to today's share of it.
    pub async fn credit(&self, id: &ExternalId, seconds: i64, day: Day) -> Result<()> {
        sqlx::query(
            "INSERT INTO ledger
                (external_id, tracked_seconds, today_day, today_seconds, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(external_id) DO UPDATE SET
                tracked_seconds = tracked_seconds + excluded.tracked_seconds,
                today_seconds = CASE
                    WHEN today_day = excluded.today_day
                    THEN today_seconds + excluded.today_seconds
                    ELSE excluded.today_seconds END,
                today_day = excluded.today_day,
                updated_at = excluded.updated_at",
        )
        .bind(id.as_str())
        .bind(seconds)
        .bind(day.to_string())
        .bind(seconds)
        .bind(Timestamp::now().seconds())
        .execute(self.db.pool())
        .await
        .map_err(failed)?;
        Ok(())
    }

    /// Marks seconds as told to the source.
    pub async fn logged(&self, id: &ExternalId, seconds: i64) -> Result<()> {
        sqlx::query(
            "UPDATE ledger SET logged_seconds = logged_seconds + ?, updated_at = ?
             WHERE external_id = ?",
        )
        .bind(seconds)
        .bind(Timestamp::now().seconds())
        .bind(id.as_str())
        .execute(self.db.pool())
        .await
        .map_err(failed)?;
        Ok(())
    }
}

/// One row as the app reads it: today's share counts only while the day stands.
fn summary(row: (String, i64, i64, String, i64), today: &str) -> (ExternalId, TimeSummary) {
    let (id, tracked, logged, day, seconds) = row;
    let summary = TimeSummary {
        tracked_seconds: tracked,
        logged_seconds: logged,
        today_seconds: match day == today {
            true => seconds,
            false => 0,
        },
        unlogged_seconds: (tracked - logged).max(0),
    };
    (ExternalId::new(id), summary)
}

fn failed(source: sqlx::Error) -> Error {
    Error::invalid(format!("the ledger could not be read or written: {source}"))
}
