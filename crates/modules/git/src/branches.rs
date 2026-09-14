use std::path::Path;

use crate::{Error, Git, Result};

impl Git {
    /// A new branch at `from`, not checked out anywhere.
    pub async fn branch_create(&self, name: &str, from: &str) -> Result<()> {
        self.text(&["branch", name, from]).await.map(drop)
    }

    /// Check out `branch` here, creating it at HEAD when asked.
    pub async fn switch(&self, branch: &str, create: bool) -> Result<()> {
        let args: &[&str] = if create {
            &["switch", "-c", branch]
        } else {
            &["switch", branch]
        };
        self.text(args).await.map(drop)
    }

    /// A worktree at `path` on an existing local branch.
    pub async fn worktree_add(&self, path: &Path, branch: &str) -> Result<()> {
        let path_str = path.to_string_lossy();
        self.worktree(&["worktree", "add", &path_str, branch], path)
            .await
    }

    /// A worktree at `path` on a new local `branch` tracking `origin/<remote_branch>`.
    pub async fn worktree_add_tracking(
        &self,
        path: &Path,
        branch: &str,
        remote_branch: &str,
    ) -> Result<()> {
        let path_str = path.to_string_lossy();
        let track = format!("origin/{remote_branch}");
        self.worktree(
            &[
                "worktree", "add", "--track", "-b", branch, &path_str, &track,
            ],
            path,
        )
        .await
    }

    /// Forget worktrees whose directory is gone.
    pub async fn worktree_prune(&self) -> Result<()> {
        self.text(&["worktree", "prune"]).await.map(drop)
    }

    async fn worktree(&self, args: &[&str], path: &Path) -> Result<()> {
        match self.text(args).await {
            Ok(_) => Ok(()),
            Err(Error::Failed { stderr, .. }) if stderr.contains("already exists") => {
                Err(Error::AlreadyExists {
                    path: path.to_path_buf(),
                })
            }
            Err(e) => Err(e),
        }
    }
}
