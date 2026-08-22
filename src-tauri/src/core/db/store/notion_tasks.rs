use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::NotionTask;

const COLUMNS: &str = "page_id, short_id, title, status, priority, synced_at";

pub async fn upsert(exec: impl SqliteExecutor<'_>, task: &NotionTask) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO notion_tasks (page_id, short_id, title, status, priority, synced_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(page_id) DO UPDATE SET
           short_id = excluded.short_id,
           title    = excluded.title,
           status   = excluded.status,
           priority = excluded.priority,
           synced_at = excluded.synced_at",
    )
    .bind(&task.page_id)
    .bind(&task.short_id)
    .bind(&task.title)
    .bind(&task.status)
    .bind(&task.priority)
    .bind(task.synced_at)
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn get_by_short_id(
    exec: impl SqliteExecutor<'_>,
    short_id: &str,
) -> StoreResult<Option<NotionTask>> {
    Ok(
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM notion_tasks WHERE short_id = ?"))
            .bind(short_id)
            .fetch_optional(exec)
            .await?,
    )
}

pub async fn all(exec: impl SqliteExecutor<'_>) -> StoreResult<Vec<NotionTask>> {
    Ok(
        sqlx::query_as(&format!("SELECT {COLUMNS} FROM notion_tasks ORDER BY synced_at DESC"))
            .fetch_all(exec)
            .await?,
    )
}

pub async fn set_status(
    exec: impl SqliteExecutor<'_>,
    short_id: &str,
    status: &str,
) -> StoreResult<()> {
    sqlx::query("UPDATE notion_tasks SET status = ? WHERE short_id = ?")
        .bind(status)
        .bind(short_id)
        .execute(exec)
        .await?;
    Ok(())
}
