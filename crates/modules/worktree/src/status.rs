//! What git says about a worktree, for the rows that show it.

use groove_git::Git;
use groove_types::{Worktree, WorktreeStatus};

use crate::{Pool, Result};

impl Pool {
    /// The counts a worktree row shows: changes from status, ahead and behind from its own
    /// head on origin, or ahead of the base when it was never pushed.
    pub async fn status(&self, worktree: &Worktree) -> Result<WorktreeStatus> {
        let git = Git::at(&worktree.path);
        let changes = git.status().await?;
        let modified = changes.iter().filter(|c| c.is_modified()).count() as u32;
        let staged = changes.iter().filter(|c| c.is_staged()).count() as u32;
        let (ahead, behind) = match git.ahead_behind(&worktree.branch).await? {
            Some(counts) => counts,
            None => match git.base_ref(worktree.base_ref.as_deref()).await {
                Ok(base) => (git.commits_since(&base).await?, 0),
                Err(_) => (0, 0),
            },
        };
        Ok(WorktreeStatus {
            modified,
            staged,
            ahead,
            behind,
        })
    }
}
