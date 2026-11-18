//! The change the one cluster write waiting on the user would make, shown in place of the worktree's.

use groove_types::ApprovalId;

use crate::{DriftKey, Prepared, State};

const REVIEW: &str = "review";

impl State {
    /// The change of `id` shown in place of the worktree's until it is decided.
    pub fn review(&mut self, id: &ApprovalId, change: Prepared) {
        self.drifted((key(id), String::new()), change);
    }

    /// The write whose change the diff shows, when it shows one.
    pub fn reviewed(&self) -> Option<&str> {
        let drift = self.drift.as_ref().filter(|one| one.app.0 == REVIEW);
        drift.map(|one| one.app.1.as_str())
    }

    /// The change of `id` let go; true when the worktree's change must be read again.
    pub fn decided(&mut self, id: &ApprovalId) -> bool {
        self.reviewed() == Some(id.as_str()) && self.undrift()
    }
}

fn key(id: &ApprovalId) -> DriftKey {
    (REVIEW.to_string(), id.as_str().to_string(), String::new())
}
