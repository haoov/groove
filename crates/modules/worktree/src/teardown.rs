use std::path::Path;

use groove_git::Git;
use groove_types::{SessionId, Worktree, WorktreeId};

use crate::{Error, Pool, Result};

impl Pool {
    /// Removes the directory and the row; the branch stays. A dirty worktree is refused unless forced.
    pub async fn close(&self, id: &WorktreeId, force: bool) -> Result<Worktree> {
        let worktree = self.worktree(id).await?;
        if !force && is_dirty(&worktree.path).await {
            return Err(Error::Dirty);
        }
        let stop_at = self.layout.session_dir(worktree.session.as_str());
        remove_dir(Path::new(&worktree.path), &stop_at)?;
        if let Ok(repo) = self.repo(&worktree.repo).await {
            let _ = Git::at(&repo.local_path).worktree_prune().await;
        }
        self.remove_worktree(id).await?;
        Ok(worktree)
    }

    /// Every worktree directory of the session, then the session directory itself.
    pub async fn cleanup_session(&self, session: &SessionId) -> Result<()> {
        let dir = self.layout.session_dir(session.as_str());
        for worktree in self.worktrees_of(session).await? {
            remove_dir(Path::new(&worktree.path), &dir)?;
            if let Ok(repo) = self.repo(&worktree.repo).await {
                let _ = Git::at(&repo.local_path).worktree_prune().await;
            }
            self.remove_worktree(&worktree.id).await?;
        }
        remove_tree(&dir)
    }
}

async fn is_dirty(path: &str) -> bool {
    Git::at(path)
        .status()
        .await
        .map(|c| !c.is_empty())
        .unwrap_or(false)
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
