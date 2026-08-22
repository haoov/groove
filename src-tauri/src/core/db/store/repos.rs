use sqlx::SqliteExecutor;

use super::super::error::{StoreError, StoreResult};
use super::super::models::Repo;

const COLUMNS: &str = "id, host, group_path, project, local_path";

pub async fn upsert(exec: impl SqliteExecutor<'_>, repo: &Repo) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO repos (id, host, group_path, project, local_path)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET local_path = excluded.local_path",
    )
    .bind(&repo.id)
    .bind(&repo.host)
    .bind(&repo.group_path)
    .bind(&repo.project)
    .bind(&repo.local_path)
    .execute(exec)
    .await?;
    Ok(())
}

pub async fn get(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Repo> {
    get_opt(exec, id)
        .await?
        .ok_or_else(|| StoreError::not_found("repo", id))
}

pub async fn get_opt(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Option<Repo>> {
    Ok(sqlx::query_as(&format!("SELECT {COLUMNS} FROM repos WHERE id = ?"))
        .bind(id)
        .fetch_optional(exec)
        .await?)
}

pub async fn attached_to(exec: impl SqliteExecutor<'_>, session_id: &str) -> StoreResult<Vec<Repo>> {
    Ok(sqlx::query_as(
        "SELECT r.id, r.host, r.group_path, r.project, r.local_path
         FROM repos r
         JOIN session_repos sr ON sr.repo_id = r.id
         WHERE sr.session_id = ?
         ORDER BY r.project",
    )
    .bind(session_id)
    .fetch_all(exec)
    .await?)
}

pub async fn attached_paths(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
) -> StoreResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT r.local_path FROM repos r
         JOIN session_repos sr ON sr.repo_id = r.id
         WHERE sr.session_id = ?",
    )
    .bind(session_id)
    .fetch_all(exec)
    .await?)
}

pub async fn attach(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    repo_id: &str,
) -> StoreResult<()> {
    sqlx::query(
        "INSERT INTO session_repos (session_id, repo_id, added_at)
         VALUES (?, ?, unixepoch())
         ON CONFLICT(session_id, repo_id) DO NOTHING",
    )
    .bind(session_id)
    .bind(repo_id)
    .execute(exec)
    .await?;
    Ok(())
}

/// Replace the session's whole repo set — the picker submits the full selection.
pub async fn set_attached(
    exec: impl SqliteExecutor<'_> + Copy,
    session_id: &str,
    repo_ids: &[String],
) -> StoreResult<()> {
    sqlx::query("DELETE FROM session_repos WHERE session_id = ?")
        .bind(session_id)
        .execute(exec)
        .await?;
    for repo_id in repo_ids {
        attach(exec, session_id, repo_id).await?;
    }
    Ok(())
}
