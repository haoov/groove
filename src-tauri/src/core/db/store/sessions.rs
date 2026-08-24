use sqlx::{SqliteExecutor, SqlitePool};

use super::super::error::{StoreError, StoreResult};
use super::super::models::{Session, SessionKind, TaskView};

const COLUMNS: &str =
    "id, kind, title, external_id, review_project, review_iid, created_at";

pub async fn get(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Session> {
    get_opt(exec, id)
        .await?
        .ok_or_else(|| StoreError::not_found("session", id))
}

pub async fn get_opt(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Option<Session>> {
    Ok(sqlx::query_as(&format!("SELECT {COLUMNS} FROM sessions WHERE id = ?"))
        .bind(id)
        .fetch_optional(exec)
        .await?)
}

/// The session as the frontend's task shape: real tasks read status/priority
/// from the mirror, synthetic sessions synthesize them.
pub async fn view(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<TaskView> {
    view_opt(exec, id)
        .await?
        .ok_or_else(|| StoreError::not_found("session", id))
}

pub async fn view_opt(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Option<TaskView>> {
    #[derive(sqlx::FromRow)]
    struct ViewRow {
        id: String,
        external_id: Option<String>,
        title: String,
        status: Option<String>,
        priority: Option<String>,
        provider: Option<String>,
        url: Option<String>,
        created_at: i64,
    }
    let row: Option<ViewRow> = sqlx::query_as(
        "SELECT s.id, s.external_id, s.title, pt.status, pt.priority, pt.provider, pt.url, s.created_at
         FROM sessions s
         LEFT JOIN provider_tasks pt ON pt.external_id = s.external_id
         WHERE s.id = ?",
    )
    .bind(id)
    .fetch_optional(exec)
    .await?;
    Ok(row.map(|r| TaskView {
        short_id: r.id,
        external_id: r.external_id.unwrap_or_default(),
        provider: r.provider,
        external_url: r.url,
        title: r.title,
        status: r.status.unwrap_or_else(|| "in_progress".to_string()),
        priority: r.priority,
        last_synced_at: r.created_at,
    }))
}

/// Open (or re-open) a session for a mirrored task.
pub async fn open_task(exec: impl SqliteExecutor<'_> + Copy, short_id: &str) -> StoreResult<Session> {
    let task = super::provider_tasks::get_by_short_id(exec, short_id)
        .await?
        .ok_or_else(|| StoreError::not_found("task", short_id))?;
    sqlx::query(
        "INSERT INTO sessions (id, kind, title, external_id, created_at)
         VALUES (?, 'task', ?, ?, unixepoch())
         ON CONFLICT(id) DO UPDATE SET title = excluded.title",
    )
    .bind(short_id)
    .bind(&task.title)
    .bind(&task.external_id)
    .execute(exec)
    .await?;
    get(exec, short_id).await
}

pub async fn create_explorer(
    exec: impl SqliteExecutor<'_> + Copy,
    id: &str,
    title: &str,
) -> StoreResult<Session> {
    sqlx::query(
        "INSERT INTO sessions (id, kind, title, created_at)
         VALUES (?, 'explorer', ?, unixepoch())",
    )
    .bind(id)
    .bind(title)
    .execute(exec)
    .await?;
    get(exec, id).await
}

/// Create or refresh the session tracking one MR review. `(project, iid)` is
/// the identity; reopening the same MR resumes the session.
pub async fn upsert_review(
    exec: impl SqliteExecutor<'_> + Copy,
    id: &str,
    project: &str,
    iid: i64,
    title: &str,
) -> StoreResult<Session> {
    sqlx::query(
        "INSERT INTO sessions (id, kind, title, review_project, review_iid, created_at)
         VALUES (?, 'review', ?, ?, ?, unixepoch())
         ON CONFLICT(review_project, review_iid)
           DO UPDATE SET title = excluded.title",
    )
    .bind(id)
    .bind(title)
    .bind(project)
    .bind(iid)
    .execute(exec)
    .await?;
    let session: Session = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM sessions WHERE review_project = ? AND review_iid = ?"
    ))
    .bind(project)
    .bind(iid)
    .fetch_one(exec)
    .await?;
    Ok(session)
}

pub async fn rename_explorer(
    exec: impl SqliteExecutor<'_>,
    id: &str,
    title: &str,
) -> StoreResult<()> {
    sqlx::query("UPDATE sessions SET title = ? WHERE id = ? AND kind = 'explorer'")
        .bind(title)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}

/// Delete a session and, through the cascades, everything it owns.
pub async fn remove(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}

/// Turn an explorer into a real task in one transaction: mirror the new task,
/// re-key the session (children follow via ON UPDATE CASCADE), and persist the
/// new branch/path of every worktree that switched.
pub async fn adopt_explorer(
    pool: &SqlitePool,
    explorer_id: &str,
    task: &super::super::models::ProviderTask,
    switched: &[(String, String)],
    new_branch: &str,
) -> StoreResult<()> {
    let mut tx = pool.begin().await?;

    super::provider_tasks::upsert(&mut *tx, task).await?;
    let updated = sqlx::query(
        "UPDATE sessions SET id = ?, kind = 'task', external_id = ?, title = ?
         WHERE id = ? AND kind = 'explorer'",
    )
    .bind(&task.short_id)
    .bind(&task.external_id)
    .bind(&task.title)
    .bind(explorer_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(StoreError::not_found("explorer session", explorer_id));
    }

    for (worktree_id, path) in switched {
        sqlx::query("UPDATE worktrees SET branch = ?, path = ? WHERE id = ?")
            .bind(new_branch)
            .bind(path)
            .bind(worktree_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Session kind without loading the row — the discriminator several commands
/// branch on.
pub async fn kind_of(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Option<SessionKind>> {
    Ok(sqlx::query_scalar("SELECT kind FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_optional(exec)
        .await?)
}
