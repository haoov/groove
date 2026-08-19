//! Removing worktrees: one, or a whole session's.

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::db::store;
use crate::core::git;

/// Delete a worktree's directory and prune its clone's registration.
/// Disk only — DB rows are the store's job.
async fn remove_worktree_dir(wt_path: String, repo_local_path: Option<String>) {
    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(wt_path)).await;
    if let Some(local_path) = repo_local_path {
        let _ = git::run(&local_path, &["worktree", "prune"]).await;
    }
}

/// Remove every worktree directory of a session and the session directory
/// itself. The DB rows cascade when the session row is deleted.
pub async fn cleanup_session_worktrees(session_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    for wt in store::worktrees::for_session(pool, session_id).await? {
        let repo = store::repos::get_opt(pool, &wt.repo_id).await?;
        remove_worktree_dir(wt.path, repo.map(|r| r.local_path)).await;
    }

    let dir = super::pool::session_dir(session_id);
    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(dir)).await;
    git::cache::flush();

    Ok(())
}

#[tauri::command]
pub async fn close_worktree(
    app: tauri::AppHandle,
    worktree_id: String,
    force: Option<bool>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    close_worktree_impl(&app, &worktree_id, force, &pool)
        .await
        .map_err(|e| e.to_string())
}

async fn close_worktree_impl(
    app: &tauri::AppHandle,
    worktree_id: &str,
    force: Option<bool>,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let wt = store::worktrees::get(pool, worktree_id).await?;

    // Guard against destroying uncommitted work: unless forced, refuse to close a
    // worktree with a dirty working tree (the frontend re-invokes with force after
    // a confirm).
    if force != Some(true) {
        let dirty = git::output(&wt.path, &["status", "--porcelain"])
            .await
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);
        if dirty {
            return Err(anyhow::anyhow!(
                "worktree has uncommitted changes — commit or discard first, or force close"
            ));
        }
    }

    let repo = store::repos::get_opt(pool, &wt.repo_id).await?;
    remove_worktree_dir(wt.path.clone(), repo.map(|r| r.local_path)).await;
    git::cache::flush();

    let closed = store::worktrees::close(pool, worktree_id).await?;

    app.emit(
        crate::events::WORKTREE_CLOSED,
        serde_json::json!({
            "worktree_id": worktree_id,
            "session_id": closed.session_id,
            "repo_id": closed.repo_id,
        }),
    )?;

    Ok(())
}
