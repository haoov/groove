//! Taking a worktree away, and the session directory with the last of them.

use std::path::Path;

use groove_git::Git;
use std::collections::BTreeSet;

use groove_types::{SessionId, Worktree, WorktreeId};

use crate::{Error, Pool, Result};

/// What a close refuses to lose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Uncommitted changes, and commits origin lacks.
    Everything,
    /// Uncommitted changes only: the branch's MR merged its commits elsewhere.
    Changes,
    Nothing,
}

impl Pool {
    /// Removes the directory, the local branch and the row; what `keep` names refuses it.
    pub async fn close(&self, id: &WorktreeId, keep: Keep) -> Result<Worktree> {
        let worktree = self.worktree(id).await?;
        self.refuse_loss(&worktree, keep).await?;
        let stop_at = self.layout.session_dir(worktree.session.as_str());
        self.tear_down(&worktree, &stop_at).await?;
        self.remove_worktree(id).await?;
        Ok(worktree)
    }

    /// Every worktree, then the directory; unforced, lost work refuses it, bar `landed` commits.
    pub async fn cleanup_session(
        &self,
        session: &SessionId,
        force: bool,
        landed: &BTreeSet<WorktreeId>,
    ) -> Result<()> {
        let dir = self.layout.session_dir(session.as_str());
        let worktrees = self.worktrees_of(session).await?;
        for worktree in &worktrees {
            let keep = match (force, landed.contains(&worktree.id)) {
                (true, _) => Keep::Nothing,
                (false, true) => Keep::Changes,
                (false, false) => Keep::Everything,
            };
            self.refuse_loss(worktree, keep).await?;
        }
        for worktree in worktrees {
            self.tear_down(&worktree, &dir).await?;
            self.remove_worktree(&worktree.id).await?;
        }
        remove_tree(&dir)
    }

    /// Refuses a worktree holding what `keep` names, or one git cannot answer for.
    async fn refuse_loss(&self, worktree: &Worktree, keep: Keep) -> Result<()> {
        if keep == Keep::Nothing {
            return Ok(());
        }
        let unknown = || Error::Unknown {
            branch: worktree.branch.clone(),
        };
        let git = match Path::new(&worktree.path).is_dir() {
            true => {
                let git = Git::at(&worktree.path);
                if !git.status().await.map_err(|_| unknown())?.is_empty() {
                    return Err(Error::Dirty);
                }
                if keep == Keep::Changes {
                    return Ok(());
                }
                git
            }
            false => match self.repo(&worktree.repo).await {
                Ok(repo) => Git::at(&repo.local_path),
                Err(_) => return Ok(()),
            },
        };
        let branch = format!("refs/heads/{}", worktree.branch);
        if !git.ref_exists(&branch).await.map_err(|_| unknown())? {
            return Ok(());
        }
        let pinned = worktree.base_ref.as_deref();
        let ahead = git.unpushed(&worktree.branch, pinned).await;
        match ahead.map_err(|_| unknown())? {
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
