//! The routine capability: the routine files and their runs.

pub mod runs;

#[cfg(test)]
mod tests;

use groove_types::{AgentStatus, Routine, SessionId, Timestamp};

pub use groove_routines as routines;

use runs::{Ending, Run, Runs};

/// The `routine` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    /// Every routine file, the team's and the user's, as the last read found them.
    pub routines: Vec<groove_routines::Listed>,
    pub runs: Runs,
}

impl State {
    /// One routine by id, when its file reads.
    pub fn routine(&self, id: &str) -> Option<&Routine> {
        let listed = self.routines.iter().find(|one| one.id == id)?;
        listed.read.as_ref().ok()
    }

    /// Every routine whose file reads.
    pub fn readable(&self) -> Vec<Routine> {
        let read = self
            .routines
            .iter()
            .filter_map(|one| one.read.as_ref().ok());
        read.cloned().collect()
    }

    /// The runs whose agent stopped, taken off; how each ended.
    pub fn ended(
        &mut self,
        status_of: impl Fn(&SessionId) -> Option<AgentStatus>,
        now: Timestamp,
    ) -> Vec<(Run, Ending)> {
        self.runs.finished(status_of, now)
    }

    /// The runs waiting their turn that the cap and their agents allow now, moved to running.
    pub fn due(
        &mut self,
        cap: usize,
        status_of: impl Fn(&SessionId) -> Option<AgentStatus>,
        now: Timestamp,
    ) -> Vec<Run> {
        let routines = &self.routines;
        let kind_of = |id: &str| {
            let listed = routines.iter().find(|one| one.id == id)?;
            listed.read.as_ref().ok().map(|one| one.kind)
        };
        self.runs.due(cap, kind_of, status_of, now)
    }
}
