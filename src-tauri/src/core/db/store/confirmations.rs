use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::PendingConfirmation;

const COLUMNS: &str = "id, session_id, op_type, payload, origin, created_at";

/// Insert a pending row. False when an identical request is already pending.
pub async fn insert(
    exec: impl SqliteExecutor<'_>,
    id: &str,
    session_id: Option<&str>,
    op_type: &str,
    payload: &str,
    origin: &str,
) -> StoreResult<bool> {
    let done = sqlx::query(
        "INSERT INTO pending_confirmations (id, session_id, op_type, payload, origin, created_at)
         VALUES (?, ?, ?, ?, ?, unixepoch())",
    )
    .bind(id)
    .bind(session_id)
    .bind(op_type)
    .bind(payload)
    .bind(origin)
    .execute(exec)
    .await;

    match done {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Atomically claim one confirmation; the row survives until its outcome is delivered.
pub async fn claim(
    exec: impl SqliteExecutor<'_>,
    id: &str,
) -> StoreResult<Option<PendingConfirmation>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE pending_confirmations SET claimed_at = unixepoch()
         WHERE id = ? AND claimed_at IS NULL RETURNING {COLUMNS}"
    ))
    .bind(id)
    .fetch_optional(exec)
    .await?)
}

/// Drop a row once its outcome is emitted.
pub async fn delete(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<()> {
    sqlx::query("DELETE FROM pending_confirmations WHERE id = ?")
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
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
         WHERE claimed_at IS NULL
           AND op_type = ? AND IFNULL(session_id, '') = IFNULL(?, '') AND payload = ?",
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
        "SELECT {COLUMNS} FROM pending_confirmations
         WHERE claimed_at IS NULL ORDER BY created_at, id"
    ))
    .fetch_all(exec)
    .await?)
}

/// Rows a run claimed but never finished — a crash between the claim and the outcome.
pub async fn claimed(exec: impl SqliteExecutor<'_>) -> StoreResult<Vec<PendingConfirmation>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM pending_confirmations
         WHERE claimed_at IS NOT NULL ORDER BY created_at, id"
    ))
    .fetch_all(exec)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::test_pool;

    #[tokio::test]
    async fn a_claim_keeps_the_row_until_its_outcome_lands() {
        let pool = test_pool().await;
        insert(&pool, "c1", None, "git.push", "{}", "ui")
            .await
            .unwrap();

        let claimed_row = claim(&pool, "c1").await.unwrap().expect("first claim wins");
        assert_eq!(claimed_row.op_type, "git.push");
        assert!(
            claim(&pool, "c1").await.unwrap().is_none(),
            "a second concurrent resolve must not run the op again"
        );

        assert!(all(&pool).await.unwrap().is_empty(), "no longer pending");
        assert_eq!(
            claimed(&pool).await.unwrap().len(),
            1,
            "a crash must leave the request to re-surface"
        );

        delete(&pool, "c1").await.unwrap();
        assert!(claimed(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_identical_pending_request_cannot_be_queued_twice() {
        let pool = test_pool().await;
        assert!(insert(&pool, "c1", None, "git.push", "{\"a\":1}", "mcp")
            .await
            .unwrap());
        assert!(
            !insert(&pool, "c2", None, "git.push", "{\"a\":1}", "mcp")
                .await
                .unwrap(),
            "the second copy must be refused"
        );
        assert_eq!(all(&pool).await.unwrap().len(), 1);

        // The identity is free again once the first is claimed and gone.
        claim(&pool, "c1").await.unwrap().unwrap();
        assert!(insert(&pool, "c3", None, "git.push", "{\"a\":1}", "mcp")
            .await
            .unwrap());
    }
}
