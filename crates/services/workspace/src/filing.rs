//! The file operations of each worktree, run in the order asked.

use groove_types::WorktreeId;
use tokio::sync::oneshot::{Receiver, Sender, channel};

/// One file operation's place in its worktree's line; dropping it lets the next one run.
#[derive(Debug)]
pub struct Filing {
    before: Option<Receiver<()>>,
    _done: Sender<()>,
}

impl Filing {
    /// Waits until the operation asked before this one has ended.
    pub async fn wait(&mut self) {
        if let Some(before) = self.before.take() {
            let _ = before.await;
        }
    }
}

impl crate::State {
    /// One more file operation on `worktree`, behind the ones out.
    pub fn filing(&mut self, worktree: &WorktreeId) -> Filing {
        let (done, last) = channel();
        let before = self.filings.insert(worktree.clone(), last);
        Filing {
            before,
            _done: done,
        }
    }
}
