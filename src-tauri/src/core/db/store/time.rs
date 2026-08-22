use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::{ActivityDay, TimeSummary};

/// Tracked seconds per calendar day, summed across every session — oldest first.
pub async fn activity(exec: impl SqliteExecutor<'_>) -> StoreResult<Vec<ActivityDay>> {
    let rows = sqlx::query_as::<_, ActivityDay>(
        "SELECT day, CAST(SUM(seconds) AS INTEGER) AS seconds
           FROM time_entries GROUP BY day ORDER BY day",
    )
    .fetch_all(exec)
    .await?;
    Ok(rows)
}

pub async fn add(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    day: &str,
    seconds: i64,
) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO time_entries (session_id, day, seconds) VALUES (?, ?, ?)
         ON CONFLICT(session_id, day) DO UPDATE SET seconds = seconds + excluded.seconds",
    )
    .bind(session_id)
    .bind(day)
    .bind(seconds)
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn log(exec: impl SqliteExecutor<'_>, session_id: &str, seconds: i64) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO time_logs (id, session_id, seconds, logged_at)
         VALUES (?, ?, ?, unixepoch())",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(session_id)
    .bind(seconds)
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn summary(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    today: &str,
) -> StoreResult<TimeSummary> {
    let (tracked, today_seconds, logged): (i64, i64, i64) = sqlx::query_as(
        "SELECT
           COALESCE((SELECT SUM(seconds) FROM time_entries WHERE session_id = ?1), 0),
           COALESCE((SELECT seconds FROM time_entries WHERE session_id = ?1 AND day = ?2), 0),
           COALESCE((SELECT SUM(seconds) FROM time_logs WHERE session_id = ?1), 0)",
    )
    .bind(session_id)
    .bind(today)
    .fetch_one(exec)
    .await?;
    Ok(TimeSummary {
        session_id: session_id.to_string(),
        tracked_seconds: tracked,
        logged_seconds: logged,
        today_seconds,
        unlogged_seconds: (tracked - logged).max(0),
    })
}
