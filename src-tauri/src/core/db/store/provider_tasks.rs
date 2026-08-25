use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::ProviderTask;

const COLUMNS: &str =
    "external_id, short_id, title, status, priority, synced_at, provider, url, board, branch_tag";

pub async fn upsert(exec: impl SqliteExecutor<'_>, task: &ProviderTask) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO provider_tasks
           (external_id, short_id, title, status, priority, synced_at, provider, url, board, branch_tag)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(external_id) DO UPDATE SET
           short_id  = excluded.short_id,
           title     = excluded.title,
           status    = excluded.status,
           priority  = excluded.priority,
           synced_at = excluded.synced_at,
           provider  = excluded.provider,
           url       = excluded.url,
           board     = excluded.board,
           branch_tag = excluded.branch_tag",
    )
    .bind(&task.external_id)
    .bind(&task.short_id)
    .bind(&task.title)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(task.synced_at)
    .bind(&task.provider)
    .bind(&task.url)
    .bind(&task.board)
    .bind(&task.branch_tag)
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn get_by_short_id(
    exec: impl SqliteExecutor<'_>,
    short_id: &str,
) -> StoreResult<Option<ProviderTask>> {
    Ok(
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM provider_tasks WHERE short_id = ?"))
            .bind(short_id)
            .fetch_optional(exec)
            .await?,
    )
}

pub async fn get_by_external_id(
    exec: impl SqliteExecutor<'_>,
    external_id: &str,
) -> StoreResult<Option<ProviderTask>> {
    Ok(
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM provider_tasks WHERE external_id = ?"))
            .bind(external_id)
            .fetch_optional(exec)
            .await?,
    )
}

/// One provider's mirrored tasks, for showing what is known when its queue is
/// unreachable.
pub async fn for_provider(
    exec: impl SqliteExecutor<'_>,
    provider: &str,
) -> StoreResult<Vec<ProviderTask>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM provider_tasks WHERE provider = ? ORDER BY synced_at DESC"
    ))
    .bind(provider)
    .fetch_all(exec)
    .await?)
}

pub async fn all(exec: impl SqliteExecutor<'_>) -> StoreResult<Vec<ProviderTask>> {
    Ok(
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM provider_tasks ORDER BY synced_at DESC"))
            .fetch_all(exec)
            .await?,
    )
}

pub async fn set_status(
    exec: impl SqliteExecutor<'_>,
    short_id: &str,
    status: &str,
) -> StoreResult<()> {
    sqlx::query("UPDATE provider_tasks SET status = ? WHERE short_id = ?")
        .bind(status)
        .bind(short_id)
        .execute(exec)
        .await?;
    Ok(())
}

/// Drop mirror rows for `provider` that the latest sync did not return, so a task
/// that left the queue stops showing up. Sessions are untouched: an open task keeps
/// its session whether or not it is still queued.
/// Every mirror row of one provider, except tasks that are checked out — used
/// when the source is disabled, since its sync loop (the usual pruner) no
/// longer runs.
pub async fn prune_provider(exec: impl SqliteExecutor<'_>, provider: &str) -> StoreResult<u64> {
    Ok(sqlx::query(
        "DELETE FROM provider_tasks
          WHERE provider = ?
            AND short_id NOT IN (SELECT id FROM sessions)",
    )
    .bind(provider)
    .execute(exec)
    .await?
    .rows_affected())
}

pub async fn prune_missing(
    exec: impl SqliteExecutor<'_>,
    provider: &str,
    keep: &[String],
) -> StoreResult<u64> {
    let holes = std::iter::repeat("?").take(keep.len()).collect::<Vec<_>>().join(",");
    let sql = format!(
        "DELETE FROM provider_tasks
          WHERE provider = ?
            AND external_id NOT IN ({holes})
            AND short_id NOT IN (SELECT id FROM sessions)"
    );
    let mut q = sqlx::query(&sql).bind(provider);
    for id in keep {
        q = q.bind(id);
    }
    Ok(q.execute(exec).await?.rows_affected())
}
