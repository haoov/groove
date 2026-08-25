//! Removing worktrees: one, or a whole session's.

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::db::store;
use crate::core::git;
use std::path::{Path, PathBuf};

/// Delete a worktree's directory and prune its clone's registration.
/// Disk only — DB rows are the store's job.
async fn remove_worktree_dir(wt_path: String, repo_local_path: Option<String>, stop_at: PathBuf) {
    let _ = tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&wt_path);
        let _ = std::fs::remove_dir_all(&path);
        prune_empty_parents(&path, &stop_at);
    })
    .await;
    if let Some(local_path) = repo_local_path {
        let _ = git::run(&local_path, &["worktree", "prune"]).await;
    }
}

/// Walk up from a removed worktree deleting directories that are now empty, until
/// `stop_at` or the first that still holds something.
///
/// A branch is nested (`<project>/<type>/<name>`), so removing one worktree leaves
/// the type directory, and often the project directory, behind as skeletons.
/// `remove_dir` only succeeds on an empty directory, which is the whole guard.
fn prune_empty_parents(removed: &Path, stop_at: &Path) {
    let mut dir = removed.parent();
    while let Some(current) = dir {
        if current == stop_at || !current.starts_with(stop_at) {
            return;
        }
        if std::fs::remove_dir(current).is_err() {
            return;
        }
        dir = current.parent();
    }
}

/// Remove every worktree directory of a session and the session directory
/// itself. The DB rows cascade when the session row is deleted.
pub async fn cleanup_session_worktrees(session_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    let stop_at = super::pool::session_dir(session_id);
    for wt in store::worktrees::for_session(pool, session_id).await? {
        let repo = store::repos::get_opt(pool, &wt.repo_id).await?;
        remove_worktree_dir(wt.path, repo.map(|r| r.local_path), stop_at.clone()).await;
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
    let stop_at = super::pool::session_dir(&wt.session_id);
    remove_worktree_dir(wt.path.clone(), repo.map(|r| r.local_path), stop_at).await;
    git::cache::flush();

    let closed = store::worktrees::close(pool, worktree_id).await?;

    app.emit(
        crate::core::events::WORKTREE_CLOSED,
        serde_json::json!({
            "worktree_id": worktree_id,
            "session_id": closed.session_id,
            "repo_id": closed.repo_id,
        }),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::prune_empty_parents;
    use std::path::PathBuf;

    struct Tmp(PathBuf);
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn tree(name: &str) -> Tmp {
        let root = std::env::temp_dir().join(format!("groove-prune-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        Tmp(root)
    }

    #[test]
    fn empty_parents_go_up_to_the_session_dir() {
        let t = tree("empty");
        let session = t.0.join("gh-groove-1");
        let wt = session.join("groove/feat/lsp-1");
        std::fs::create_dir_all(&wt).unwrap();

        std::fs::remove_dir_all(&wt).unwrap();
        prune_empty_parents(&wt, &session);

        assert!(!session.join("groove").exists(), "skeleton left behind");
        assert!(session.is_dir(), "the session dir itself must survive");
    }

    /// A sibling worktree still on disk stops the walk dead.
    #[test]
    fn a_parent_that_still_holds_something_is_kept() {
        let t = tree("sibling");
        let session = t.0.join("gh-groove-1");
        let gone = session.join("groove/feat/lsp-1");
        let kept = session.join("groove/fix/parser-2");
        std::fs::create_dir_all(&gone).unwrap();
        std::fs::create_dir_all(&kept).unwrap();

        std::fs::remove_dir_all(&gone).unwrap();
        prune_empty_parents(&gone, &session);

        assert!(!session.join("groove/feat").exists(), "the emptied branch dir should go");
        assert!(kept.is_dir(), "the sibling must survive");
    }

    /// Never climb above the floor, whatever it is handed.
    #[test]
    fn the_walk_never_escapes_the_session_dir() {
        let t = tree("escape");
        let session = t.0.join("gh-groove-1");
        std::fs::create_dir_all(&session).unwrap();

        prune_empty_parents(&session.join("groove"), &session);

        assert!(session.is_dir());
        assert!(t.0.is_dir(), "must not have climbed past the floor");
    }
}
