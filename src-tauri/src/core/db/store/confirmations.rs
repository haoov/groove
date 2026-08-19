use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::PendingConfirmation;

const COLUMNS: &str = "id, session_id, op_type, payload, origin, created_at";

pub async fn insert(
    exec: impl SqliteExecutor<'_>,
    id: &str,
    session_id: Option<&str>,
    op_type: &str,
    payload: &str,
    origin: &str,
) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO pending_confirmations (id, session_id, op_type, payload, origin, created_at)
         VALUES (?, ?, ?, ?, ?, unixepoch())",
    )
    .bind(id)
    .bind(session_id)
    .bind(op_type)
    .bind(payload)
    .bind(origin)
    .execute(exec)
    .await?;
    Ok(())
}

/// Atomically claim one confirmation, so two concurrent resolves can never
/// execute the same op twice.
pub async fn claim(
    exec: impl SqliteExecutor<'_>,
    id: &str,
) -> StoreResult<Option<PendingConfirmation>> {
    Ok(sqlx::query_as(&format!(
        "DELETE FROM pending_confirmations WHERE id = ? RETURNING {COLUMNS}"
    ))
    .bind(id)
    .fetch_optional(exec)
    .await?)
}

/// Whether an identical request is already awaiting the user's decision.
pub async fn identical_pending(
    exec: impl SqliteExecutor<'_>,
    op_type: &str,
    session_id: Option<&str>,
    payload: &str,
) -> StoreResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pending_confirmations
         WHERE op_type = ? AND IFNULL(session_id, '') = IFNULL(?, '') AND payload = ?",
    )
    .bind(op_type)
    .bind(session_id)
    .bind(payload)
    .fetch_one(exec)
    .await?;
    Ok(count > 0)
}

pub async fn all(exec: impl SqliteExecutor<'_>) -> StoreResult<Vec<PendingConfirmation>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM pending_confirmations ORDER BY created_at, id"
    ))
    .fetch_all(exec)
    .await?)
}
