use sqlx::{SqliteExecutor, SqlitePool};

use super::super::error::{StoreError, StoreResult};
use super::super::models::Worktree;

const COLUMNS: &str = "id, session_id, repo_id, branch, path, base_ref, created_at";

pub async fn get(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<Worktree> {
    sqlx::query_as(&format!("SELECT {COLUMNS} FROM worktrees WHERE id = ?"))
        .bind(id)
        .fetch_optional(exec)
        .await?
        .ok_or_else(|| StoreError::not_found("worktree", id))
}

pub async fn for_session(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
) -> StoreResult<Vec<Worktree>> {
    Ok(sqlx::query_as(&format!("SELECT {COLUMNS} FROM worktrees WHERE session_id = ?"))
        .bind(session_id)
        .fetch_all(exec)
        .await?)
}

// Production caller arrives with the worktrees phase (multi-worktree pickers).
#[allow(dead_code)]
pub async fn for_repo(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    repo_id: &str,
) -> StoreResult<Vec<Worktree>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM worktrees WHERE session_id = ? AND repo_id = ? ORDER BY created_at"
    ))
    .bind(session_id)
    .bind(repo_id)
    .fetch_all(exec)
    .await?)
}

async fn for_branch(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    repo_id: &str,
    branch: &str,
) -> StoreResult<Option<Worktree>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM worktrees
         WHERE session_id = ? AND repo_id = ? AND branch = ?"
    ))
    .bind(session_id)
    .bind(repo_id)
    .bind(branch)
    .fetch_optional(exec)
    .await?)
}

pub async fn upsert(
    exec: impl SqliteExecutor<'_> + Copy,
    session_id: &str,
    repo_id: &str,
    branch: &str,
    path: &str,
) -> StoreResult<Worktree> {
    sqlx::query(
        "INSERT INTO worktrees (id, session_id, repo_id, branch, path, created_at)
         VALUES (?, ?, ?, ?, ?, unixepoch())
         ON CONFLICT(session_id, repo_id, branch) DO UPDATE SET
           path = excluded.path",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(session_id)
    .bind(repo_id)
    .bind(branch)
    .bind(path)
    .execute(exec)
    .await?;
    for_branch(exec, session_id, repo_id, branch)
        .await?
        .ok_or_else(|| StoreError::not_found("worktree", format!("{session_id}/{repo_id}@{branch}")))
}

pub async fn set_base_ref(
    exec: impl SqliteExecutor<'_>,
    id: &str,
    base_ref: &str,
) -> StoreResult<()> {
    sqlx::query("UPDATE worktrees SET base_ref = ? WHERE id = ?")
        .bind(base_ref)
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}

/// Drop the worktree row (its MRs cascade) and detach the repo from the
/// session when this was its last worktree — one transaction, one invariant.
pub async fn close(pool: &SqlitePool, id: &str) -> StoreResult<Worktree> {
    let mut tx = pool.begin().await?;
    let worktree = get(&mut *tx, id).await?;
    sqlx::query("DELETE FROM worktrees WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM session_repos
         WHERE session_id = ? AND repo_id = ?
           AND NOT EXISTS (SELECT 1 FROM worktrees
                           WHERE session_id = ? AND repo_id = ?)",
    )
    .bind(&worktree.session_id)
    .bind(&worktree.repo_id)
    .bind(&worktree.session_id)
    .bind(&worktree.repo_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(worktree)
}
