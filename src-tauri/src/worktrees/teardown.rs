//! Removing worktrees: one, or a whole session's.

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::db::store;
use crate::core::git;
use std::path::{Path, PathBuf};

/// Delete a worktree's directory and prune its clone's registration. Disk only.
async fn remove_worktree_dir(
    wt_path: String,
    repo_local_path: Option<String>,
    stop_at: PathBuf,
) -> anyhow::Result<()> {
    let removed = PathBuf::from(&wt_path);
    tokio::task::spawn_blocking(move || {
        remove_tree(&removed)?;
        prune_empty_parents(&removed, &stop_at);
        Ok::<(), std::io::Error>(())
    })
    .await?
    .map_err(|e| anyhow::anyhow!("could not remove worktree directory {wt_path}: {e}"))?;

    if let Some(local_path) = repo_local_path {
        let _ = git::run(&local_path, &["worktree", "prune"]).await;
    }
    Ok(())
}

/// Delete a tree. An absent directory is success; anything else is an error the caller must see.
fn remove_tree(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Remove empty parents of `removed` up to `stop_at`. Keep `remove_dir`: it refuses
/// a non-empty directory, which ends the walk.
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

/// Remove every worktree directory of a session and the session directory itself.
pub async fn cleanup_session_worktrees(session_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    let stop_at = super::pool::session_dir(session_id);
    for wt in store::worktrees::for_session(pool, session_id).await? {
        let repo_local = store::repos::get_opt(pool, &wt.repo_id)
            .await?
            .map(|r| r.local_path);
        remove_worktree_dir(wt.path, repo_local, stop_at.clone()).await?;
    }

    let dir = super::pool::session_dir(session_id);
    let reported = dir.clone();
    tokio::task::spawn_blocking(move || remove_tree(&dir))
        .await?
        .map_err(|e| anyhow::anyhow!("could not remove {}: {e}", reported.display()))?;
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

    // Refuse to close a dirty worktree unless forced.
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
    remove_worktree_dir(wt.path.clone(), repo.map(|r| r.local_path), stop_at).await?;
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
    use super::{prune_empty_parents, remove_worktree_dir};
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

    /// A clone with one commit on `main`, ready for `git worktree add`.
    async fn clone_fixture(name: &str) -> (Tmp, String) {
        let t = tree(name);
        let repo = t.0.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let repo_s = repo.to_string_lossy().to_string();
        git(&repo_s, &["init", "--initial-branch=main", "."]).await;
        std::fs::write(repo.join("a.txt"), "one\n").unwrap();
        git(&repo_s, &["add", "."]).await;
        git(&repo_s, &["commit", "-m", "first"]).await;
        (t, repo_s)
    }

    async fn git(dir: &str, args: &[&str]) -> String {
        let mut full = vec![
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=T",
            "-c",
            "commit.gpgsign=false",
        ];
        full.extend_from_slice(args);
        crate::core::git::run(dir, &full)
            .await
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"))
            .trim()
            .to_string()
    }

    #[tokio::test]
    async fn a_directory_that_cannot_be_removed_fails_the_teardown() {
        let t = tree("undeletable");
        let session = t.0.join("gh-groove-1");
        let wt = session.join("groove/feat/lsp-1");
        std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
        // A non-directory in the worktree's place: `remove_dir_all` cannot take it.
        std::fs::write(&wt, "not a directory").unwrap();

        let err = remove_worktree_dir(wt.to_string_lossy().to_string(), None, session.clone())
            .await
            .unwrap_err()
            .to_string();

        assert!(err.contains("could not remove worktree directory"), "{err}");
        assert!(wt.exists(), "the path is still there");
    }

    #[tokio::test]
    async fn a_worktree_that_is_already_gone_is_not_an_error() {
        let t = tree("absent");
        let session = t.0.join("gh-groove-1");
        std::fs::create_dir_all(&session).unwrap();

        remove_worktree_dir(
            session.join("groove/feat/x").to_string_lossy().to_string(),
            None,
            session,
        )
        .await
        .unwrap();
    }

    /// Teardown removes directories. Branches are the user's; it never deletes one.
    #[tokio::test]
    async fn teardown_leaves_the_local_branch_alone() {
        let (t, repo) = clone_fixture("branch").await;
        let session = t.0.join("gh-groove-1");
        let wt = session.join("repo/feat/unpushed");
        std::fs::create_dir_all(wt.parent().unwrap()).unwrap();
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "feat/unpushed",
                &wt.to_string_lossy(),
                "main",
            ],
        )
        .await;
        std::fs::write(wt.join("only-here.txt"), "work").unwrap();
        let wt_s = wt.to_string_lossy().to_string();
        git(&wt_s, &["add", "-A"]).await;
        git(&wt_s, &["commit", "-m", "unpushed work"]).await;

        remove_worktree_dir(wt_s, Some(repo.clone()), session)
            .await
            .unwrap();

        assert!(
            !git(&repo, &["branch", "--list", "feat/unpushed"])
                .await
                .is_empty(),
            "teardown deleted a branch"
        );
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

        assert!(
            !session.join("groove/feat").exists(),
            "the emptied branch dir should go"
        );
        assert!(kept.is_dir(), "the sibling must survive");
    }

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
