//! A headless agent: the tools it waits to be let run, the turn its stream says ended, and what
//! the stream adds to its conversation.

use groove_types::chat::Permission;
use groove_types::{AgentStatus, SessionId, Timestamp};

use crate::{Agent, State, Stream, StreamHeard, Terminal};

/// What one launch started: a terminal or a stream.
#[derive(Debug)]
pub enum Run {
    Terminal(Terminal),
    Stream(Stream),
}

impl Run {
    pub fn terminate(&self) {
        match self {
            Run::Terminal(one) => _ = one.terminate(),
            Run::Stream(one) => _ = one.terminate(),
        }
    }

    pub(crate) fn split(run: Option<Run>) -> (Option<Terminal>, Option<Stream>) {
        match run {
            Some(Run::Terminal(one)) => (Some(one), None),
            Some(Run::Stream(one)) => (None, Some(one)),
            None => (None, None),
        }
    }

    /// What an agent ran on, to end.
    pub fn of(agent: Agent) -> Option<Run> {
        let stream = agent.stream.map(Run::Stream);
        stream.or(agent.terminal.map(Run::Terminal))
    }
}

impl State {
    pub fn stream(&self, session: &SessionId) -> Option<&Stream> {
        self.agent(session)?.stream.as_ref()
    }

    pub fn permissions(&self, session: &SessionId) -> &[Permission] {
        let agent = self.agent(session);
        agent
            .map(|one| one.permissions.as_slice())
            .unwrap_or_default()
    }

    /// The ask `id` taken off its agent, which works again.
    pub fn answered(
        &mut self,
        session: &SessionId,
        id: &str,
        now: Timestamp,
    ) -> Option<Permission> {
        let agent = self.agent_mut(session)?;
        let at = agent.permissions.iter().position(|one| one.id == id)?;
        let taken = agent.permissions.remove(at);
        if agent.permissions.is_empty() {
            set(agent, AgentStatus::Working, now);
        }
        Some(taken)
    }

    pub fn chose(&mut self, session: &SessionId, model: String) {
        if let Some(agent) = self.agent_mut(session) {
            agent.chosen = Some(model);
        }
    }

    fn agent_mut(&mut self, session: &SessionId) -> Option<&mut Agent> {
        let found = self.agents.iter_mut().find(|(id, _)| id == session);
        found.map(|(_, one)| one)
    }
}

/// One record of the stream, when the launch it came from is still the session's.
pub(crate) fn heard(
    state: &mut State,
    (session, launch): (&SessionId, u64),
    heard: StreamHeard,
    at: Timestamp,
) {
    let Some(agent) = state.agent_mut(session).filter(|one| one.launch == launch) else {
        return;
    };
    match heard {
        StreamHeard::Asks(one) => {
            agent.permissions.push(one);
            set(agent, AgentStatus::Asking, at);
        }
        StreamHeard::Withdrawn(id) => agent.permissions.retain(|one| one.id != id),
        StreamHeard::Chat(one) => state.streamed(session, one),
        StreamHeard::Commands(listed) => agent.commands = listed,
        StreamHeard::Limits { limits, refused } => {
            agent.limits = limits;
            if let Some(text) = refused {
                state.streamed(session, groove_types::chat::Heard::Failed(text));
            }
        }
        StreamHeard::Settled { windows } => {
            for (model, window) in windows {
                agent.windows.retain(|(one, _)| *one != model);
                agent.windows.push((model, window));
            }
            agent.permissions.clear();
            let busy = matches!(
                agent.activity.status,
                AgentStatus::Working | AgentStatus::Asking
            );
            if busy {
                set(agent, AgentStatus::Idle, at);
            }
        }
    }
}

fn set(agent: &mut Agent, status: AgentStatus, at: Timestamp) {
    if agent.activity.status != status {
        agent.activity.changed_at = at;
    }
    agent.activity.status = status;
}
