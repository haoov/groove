//! The agent slice read for the routines: a routine by id, the sessions whose agent is in a state.

use std::collections::BTreeSet;

use groove_types::{AgentStatus, Routine, SessionId};

use crate::State;

impl State {
    /// One routine by id, when its file reads.
    pub fn routine(&self, id: &str) -> Option<&Routine> {
        let listed = self.routines.iter().find(|one| one.id == id)?;
        listed.read.as_ref().ok()
    }

    /// Every routine whose file reads.
    pub fn readable_routines(&self) -> Vec<Routine> {
        let read = self
            .routines
            .iter()
            .filter_map(|one| one.read.as_ref().ok());
        read.cloned().collect()
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

impl State {
    /// The runs whose agent stopped, taken off; how each ended.
    pub fn runs_ended(
        &mut self,
        now: groove_types::Timestamp,
    ) -> Vec<(crate::runs::Run, crate::runs::Ending)> {
        let agents = &self.agents;
        self.runs.finished(|id| status(agents, id), now)
    }

    /// The runs waiting their turn that the cap and their agents allow now, moved to running.
    pub fn runs_due(&mut self, cap: usize, now: groove_types::Timestamp) -> Vec<crate::runs::Run> {
        let (agents, routines) = (&self.agents, &self.routines);
        let kind_of = |id: &str| {
            let listed = routines.iter().find(|one| one.id == id)?;
            listed.read.as_ref().ok().map(|one| one.kind)
        };
        self.runs.due(cap, kind_of, |id| status(agents, id), now)
    }
}

fn status(agents: &[(SessionId, crate::Agent)], id: &SessionId) -> Option<AgentStatus> {
    let held = agents.iter().find(|(one, _)| one == id);
    held.map(|(_, agent)| agent.activity.status.clone())
}

/// One row of a session's skills menu: the skill, what it is sent with, and what the row says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillOffer {
    pub id: String,
    pub args: Option<String>,
    pub label: String,
}

impl State {
    /// A session's skills menu: no `create-task` or `promote-fact`, `convert-explorer` once a source when several.
    pub fn offers(
        &self,
        kind: &groove_types::SessionKind,
        sources: &[groove_types::ProviderId],
    ) -> Vec<SkillOffer> {
        let skills = self.skills_for(kind).into_iter();
        skills.flat_map(|one| offered(one, sources)).collect()
    }
}

fn offered(skill: &groove_types::Skill, sources: &[groove_types::ProviderId]) -> Vec<SkillOffer> {
    let row = |args: Option<String>, label: String| SkillOffer {
        id: skill.id.clone(),
        args,
        label,
    };
    match skill.name.as_str() {
        "create-task" | "promote-fact" => Vec::new(),
        "convert-explorer" if sources.len() > 1 => sources
            .iter()
            .map(|one| {
                row(
                    Some(one.as_str().to_string()),
                    format!("{} in {}", skill.label, one.label()),
                )
            })
            .collect(),
        _ => vec![row(None, skill.label.clone())],
    }
}
