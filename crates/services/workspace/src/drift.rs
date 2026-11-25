//! An Argo CD Application's drift, standing in for the worktree's change unless a cluster write does.

use crate::{Prepared, Source, State};

/// The Application, by its hub context, namespace and name.
pub type DriftKey = (String, String, String);

/// The drift read out, and the last read that failed with what it said, each by Application and version.
#[derive(Debug, Default)]
pub struct DriftRead {
    pub asking: Option<(DriftKey, String)>,
    pub failed: Option<(DriftKey, String, String)>,
}

impl State {
    /// The Application whose drift stands in, and the version it was read at.
    pub fn drift(&self) -> Option<(&DriftKey, &str)> {
        match &self.stand_in.as_ref()?.source {
            Source::Drift { app, at } => Some((app, at)),
            Source::Write(_) => None,
        }
    }

    /// The drift shown in place of the worktree's change; false while a cluster write stands there.
    pub fn drifted(&mut self, (app, at): (DriftKey, String), prepared: Prepared) -> bool {
        if self.reviewed().is_some() {
            return false;
        }
        self.show_instead(Source::Drift { app, at }, prepared);
        true
    }

    /// The drift let go, a cluster write kept; true when the worktree's change must be read again.
    pub fn undrift(&mut self) -> bool {
        self.drift().is_some() && self.show_worktree()
    }
}
