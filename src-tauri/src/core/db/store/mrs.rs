use sqlx::SqliteExecutor;

use super::super::error::{StoreError, StoreResult};
use super::super::models::Mr;

const COLUMNS: &str = "id, worktree_id, platform, remote_id, url, state";

pub async fn get(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Mr> {
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM mrs WHERE id = ?"))
        .bind(id)
        .fetch_optional(exec)
        .await?
        .ok_or_else(|| StoreError::not_found("mr", id))
}

pub async fn for_worktree(exec: impl SqliteExecutor<'_>, worktree_id: &str) -> StoreResult<Vec<Mr>> {
    Ok(sqlx::query_as(&format!("SELECT {COLUMNS} FROM mrs WHERE worktree_id = ?"))
        .bind(worktree_id)
        .fetch_all(exec)
        .await?)
}

pub async fn latest_for_worktree(
    exec: impl SqliteExecutor<'_>,
    worktree_id: &str,
) -> StoreResult<Option<Mr>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM mrs WHERE worktree_id = ? ORDER BY rowid DESC LIMIT 1"
    ))
    .bind(worktree_id)
    .fetch_optional(exec)
    .await?)
}

pub async fn upsert(
    exec: impl SqliteExecutor<'_> + Copy,
    worktree_id: &str,
    platform: &str,
    remote_id: &str,
    url: &str,
    state: &str,
) -> StoreResult<Mr> {
    sqlx::query(
        "INSERT INTO mrs (id, worktree_id, platform, remote_id, url, state)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(worktree_id, remote_id) DO UPDATE SET
           url   = excluded.url,
           state = excluded.state",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(worktree_id)
    .bind(platform)
    .bind(remote_id)
    .bind(url)
    .bind(state)
    .execute(exec)
    .await?;
    sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM mrs WHERE worktree_id = ? AND remote_id = ?"
    ))
    .bind(worktree_id)
    .bind(remote_id)
    .fetch_optional(exec)
    .await?
    .ok_or_else(|| StoreError::not_found("mr", format!("{worktree_id}/{remote_id}")))
}

pub async fn set_state(exec: impl SqliteExecutor<'_>, id: &str, state: &str) -> StoreResult<()> {
    sqlx::query("UPDATE mrs SET state = ? WHERE id = ?")
        .bind(state)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}
