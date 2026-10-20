//! The file operations of each worktree, run in the order asked.

use groove_sync::Turn;
use groove_types::WorktreeId;

impl crate::State {
    /// One more file operation on `worktree`, behind the ones out; dropping it lets the next run.
    pub fn filing(&mut self, worktree: &WorktreeId) -> Turn {
        Turn::behind(|end| self.filings.insert(worktree.clone(), end))
    }
}
