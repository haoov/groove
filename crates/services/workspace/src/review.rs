//! The change the one cluster write waiting on the user would make, shown in place of the worktree's.

use groove_types::ApprovalId;

use crate::{Prepared, Source, State};

impl State {
    /// The change of `id` shown in place of the worktree's, or of a drift, until it is decided.
    pub fn review(&mut self, id: &ApprovalId, change: Prepared) {
        self.show_instead(Source::Write(id.clone()), change);
    }

    /// The write whose change the diff shows, when it shows one.
    pub fn reviewed(&self) -> Option<&str> {
        match &self.stand_in.as_ref()?.source {
            Source::Write(id) => Some(id.as_str()),
            Source::Drift { .. } => None,
        }
    }

    /// The change of `id` let go; true when the worktree's change must be read again.
    pub fn decided(&mut self, id: &ApprovalId) -> bool {
        self.reviewed() == Some(id.as_str()) && self.show_worktree()
    }
}
