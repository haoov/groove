//! Taking a worktree away, and the session directory with the last of them.

use std::path::Path;

use groove_git::Git;
use groove_types::{SessionId, Worktree, WorktreeId};

use crate::{Error, Pool, Result};

impl Pool {
    /// Removes the directory, the local branch and the row. Uncommitted or unpushed work
    /// is refused unless forced; origin's copy of the branch is never touched.
    pub async fn close(&self, id: &WorktreeId, force: bool) -> Result<Worktree> {
        let worktree = self.worktree(id).await?;
        if !force {
            self.refuse_loss(&worktree).await?;
        }
        let stop_at = self.layout.session_dir(worktree.session.as_str());
        self.tear_down(&worktree, &stop_at).await?;
        self.remove_worktree(id).await?;
        Ok(worktree)
    }

    /// Every worktree of the session, its directory and its branch, then the session
    /// directory. Unforced, it refuses to lose work that is not committed or pushed.
    pub async fn cleanup_session(&self, session: &SessionId, force: bool) -> Result<()> {
        let dir = self.layout.session_dir(session.as_str());
        let worktrees = self.worktrees_of(session).await?;
        if !force {
            for worktree in &worktrees {
                self.refuse_loss(worktree).await?;
            }
        }
        for worktree in worktrees {
            self.tear_down(&worktree, &dir).await?;
            self.remove_worktree(&worktree.id).await?;
        }
        remove_tree(&dir)
    }

    /// Refuses a worktree holding work origin lacks, or one git cannot answer for.
    async fn refuse_loss(&self, worktree: &Worktree) -> Result<()> {
        if !Path::new(&worktree.path).is_dir() {
            return Ok(());
        }
        let git = Git::at(&worktree.path);
        let unknown = || Error::Unknown {
            branch: worktree.branch.clone(),
        };
        if !git.status().await.map_err(|_| unknown())?.is_empty() {
            return Err(Error::Dirty);
        }
        match unpushed(&git, worktree).await.ok_or_else(unknown)? {
            0 => Ok(()),
            ahead => Err(Error::Unpushed {
                branch: worktree.branch.clone(),
                ahead,
            }),
        }
    }

    /// The directory and its empty parents, the clone's registration, the local branch.
    async fn tear_down(&self, worktree: &Worktree, stop_at: &Path) -> Result<()> {
        remove_dir(Path::new(&worktree.path), stop_at)?;
        if let Ok(repo) = self.repo(&worktree.repo).await {
            let clone = Git::at(&repo.local_path);
            let _ = clone.worktree_prune().await;
            let _ = clone.branch_delete(&worktree.branch).await;
        }
        Ok(())
    }
}

/// Commits origin lacks: past the branch on origin, else past its base.
async fn unpushed(git: &Git, worktree: &Worktree) -> Option<u32> {
    if let Some((ahead, _)) = git.ahead_behind(&worktree.branch).await.ok()? {
        return Some(ahead);
    }
    let base = git.base_ref(worktree.base_ref.as_deref()).await.ok()?;
    git.commits_since(&base).await.ok()
}

/// Deletes the tree and the empty parents above it, up to `stop_at`.
fn remove_dir(path: &Path, stop_at: &Path) -> Result<()> {
    remove_tree(path)?;
    let mut dir = path.parent();
    while let Some(current) = dir {
        if current == stop_at
            || !current.starts_with(stop_at)
            || std::fs::remove_dir(current).is_err()
        {
            break;
        }
        dir = current.parent();
    }
    Ok(())
}

/// An absent directory is success.
fn remove_tree(path: &Path) -> Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}
