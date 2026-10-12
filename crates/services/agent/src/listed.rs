//! The agent slice read for the routines: the status each agent stands in.

use std::collections::BTreeSet;

use groove_types::{AgentStatus, SessionId};

use crate::State;

impl State {
    /// The status `id`'s agent stands in, when it has one.
    pub fn status(&self, id: &SessionId) -> Option<AgentStatus> {
        let held = self.agents.iter().find(|(one, _)| one == id);
        held.map(|(_, agent)| agent.activity.status.clone())
    }

    /// The sessions whose agent stands in a status `which` keeps.
    pub fn sessions_where(&self, which: impl Fn(&AgentStatus) -> bool) -> BTreeSet<SessionId> {
        let held = self
            .agents
            .iter()
            .filter(|(_, agent)| which(&agent.activity.status));
        held.map(|(id, _)| id.clone()).collect()
    }
}
