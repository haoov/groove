//! What the forge says about the selected worktree, and the poll that keeps it fresh.

use std::collections::BTreeSet;

use groove_types::{Mr, Timestamp, WorktreeId};

use crate::{Delivered, Snapshot};

/// The selected worktree's MR: the row, and the forge's last answer about it.
#[derive(Debug, Default)]
pub struct Delivery {
    pub mr: Option<Mr>,
    pub read: Option<Snapshot>,
    /// The last read failed; what stands here is older than it looks.
    pub stale: bool,
}

impl Delivery {
    /// What one read brought back.
    pub fn taken(&mut self, delivered: Delivered) {
        self.mr = Some(delivered.mr);
        self.read = Some(delivered.read);
        self.stale = false;
    }

    /// The forge has no MR for the worktree.
    pub fn none(&mut self) {
        *self = Self::default();
    }

    /// A read that failed leaves what stands, and ages it.
    pub fn aged(&mut self) {
        self.stale = true;
    }
}

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

    /// Forgets what was asked about one worktree, for a branch that just got an MR.
    pub fn forget(&mut self, worktree: &WorktreeId) {
        self.asked.remove(worktree);
    }
}
