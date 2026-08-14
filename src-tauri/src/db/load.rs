//! The row loads every module needs, written once.
//!
//! These four queries appeared 45+ times across nine files, each with its own
//! `.map_err(|e| e.to_string())`, and each surfacing sqlx's bare `RowNotFound` when
//! the row was missing — a message with no id in it, which says nothing about which
//! worktree or repo went away. The helpers name the thing instead.
//!
//! `*_opt` variants are for callers that treat a missing row as a normal answer.

use sqlx::SqlitePool;

use super::schema::{Repo, Task, Worktree};

/// One worktree by id. Errors when it is gone (a stale row, or a closed session).
pub async fn worktree(pool: &SqlitePool, id: &str) -> anyhow::Result<Worktree> {
    sqlx::query_as("SELECT * FROM worktrees WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no worktree {id}"))
}

/// Every active worktree of a task, newest first is NOT guaranteed — callers that
/// care sort themselves.
pub async fn active_worktrees(pool: &SqlitePool, task_id: &str) -> anyhow::Result<Vec<Worktree>> {
    Ok(sqlx::query_as("SELECT * FROM worktrees WHERE task_id = ? AND is_active = 1")
        .bind(task_id)
        .fetch_all(pool)
        .await?)
}

/// Every worktree row of a task, including inactive ones — used by teardown and
/// conversion, which must see rows the session view hides.
pub async fn all_worktrees(pool: &SqlitePool, task_id: &str) -> anyhow::Result<Vec<Worktree>> {
    Ok(sqlx::query_as("SELECT * FROM worktrees WHERE task_id = ?")
        .bind(task_id)
        .fetch_all(pool)
        .await?)
}

/// The worktree a task has for one repo, if any.
pub async fn worktree_for_repo(
    pool: &SqlitePool,
    task_id: &str,
    repo_id: &str,
) -> anyhow::Result<Option<Worktree>> {
    Ok(sqlx::query_as("SELECT * FROM worktrees WHERE task_id = ? AND repo_id = ?")
        .bind(task_id)
        .bind(repo_id)
        .fetch_optional(pool)
        .await?)
}

pub async fn repo(pool: &SqlitePool, id: &str) -> anyhow::Result<Repo> {
    repo_opt(pool, id).await?.ok_or_else(|| anyhow::anyhow!("no repo {id}"))
}

pub async fn repo_opt(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Repo>> {
    Ok(sqlx::query_as("SELECT * FROM repos WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn task(pool: &SqlitePool, short_id: &str) -> anyhow::Result<Task> {
    task_opt(pool, short_id).await?.ok_or_else(|| anyhow::anyhow!("no task {short_id}"))
}

pub async fn task_opt(pool: &SqlitePool, short_id: &str) -> anyhow::Result<Option<Task>> {
    Ok(sqlx::query_as("SELECT * FROM tasks WHERE short_id = ?")
        .bind(short_id)
        .fetch_optional(pool)
        .await?)
}
