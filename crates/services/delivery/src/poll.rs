//! When the forge is asked about a worktree again.

use std::collections::BTreeSet;

use groove_types::{Timestamp, WorktreeId};

/// How often an open MR is read again, in seconds.
pub const INTERVAL: i64 = 60;

/// The poll's own state: when it last ran, what it has asked about, what is out.
#[derive(Debug, Default)]
pub struct Polling {
    at: Option<Timestamp>,
    asked: BTreeSet<WorktreeId>,
    out: BTreeSet<WorktreeId>,
}

impl Polling {
    /// Whether `every` seconds have passed since the last pass.
    pub fn due(&self, now: Timestamp, every: i64) -> bool {
        match self.at {
            Some(at) => now.seconds() - at.seconds() >= every,
            None => true,
        }
    }

    pub fn ran(&mut self, now: Timestamp) {
        self.at = Some(now);
    }

    /// The window came back: the next tick asks about everything again.
    pub fn woke(&mut self) {
        self.at = None;
        self.asked.clear();
    }

    /// Whether this worktree is one to ask about: never asked, and none out for it.
    pub fn asks(&self, worktree: &WorktreeId) -> bool {
        !self.asked.contains(worktree) && !self.out.contains(worktree)
    }

    /// Whether its forge has answered, so no MR on the row means it has none.
    pub fn knows(&self, worktree: &WorktreeId) -> bool {
        self.asked.contains(worktree) && !self.out.contains(worktree)
    }

    /// Whether a read is out for it.
    pub fn is_out(&self, worktree: &WorktreeId) -> bool {
        self.out.contains(worktree)
    }

    pub fn sent(&mut self, worktree: &WorktreeId) {
        self.asked.insert(worktree.clone());
        self.out.insert(worktree.clone());
    }

    pub fn answered(&mut self, worktree: &WorktreeId) {
        self.out.remove(worktree);
    }

    /// Forgets that this worktree was asked about.
    pub fn forget(&mut self, worktree: &WorktreeId) {
        self.asked.remove(worktree);
    }
}
