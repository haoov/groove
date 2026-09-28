//! The agent capability. One agent per open session: its terminal and its activity.

pub(crate) mod launch;

#[cfg(test)]
mod tests;

use groove_types::{
    AgentStatus, Approval, ApprovalId, Error, HookKind, SessionActivity, SessionId, Timestamp,
    ToolCall,
};

pub use groove_agent_launch::{forget, hand_over};
pub use groove_approvals::{New as NewAsk, Queue};
pub use groove_hooks::{Post, Receiver};
pub use groove_mcp::{Answer, Call, Reply, Server};
pub use groove_skills as skills;
pub use groove_terminal::{Hooks, PtySpec, Select, Terminal};
pub use groove_tools as tools;
pub use groove_types::Screen;
pub use launch::{LaunchPaths, claude_bin, launch, login, palette};

/// One session's agent. `terminal` is `None` when the launch failed.
#[derive(Debug)]
pub struct Agent {
    pub terminal: Option<Terminal>,
    pub activity: SessionActivity,
    /// When this agent was launched, which says whether a skill is newer than it.
    pub started_at: Timestamp,
}

/// The `agent` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub agents: Vec<(SessionId, Agent)>,
    /// Every skill both plugins offer, as the last read found them.
    pub skills: Vec<groove_types::Skill>,
    /// The writes waiting on the user, each holding the answer it owes its agent.
    asks: Queue<Reply>,
    /// The sign-in Setup runs, while it runs.
    pub login: Option<Terminal>,
}

impl State {
    /// The skills a session of this kind offers, in the order they are listed.
    pub fn skills_for(&self, kind: &groove_types::SessionKind) -> Vec<&groove_types::Skill> {
        self.skills
            .iter()
            .filter(|one| one.offered_to(kind))
            .collect()
    }

    /// Whether a skill changed after this session's agent started.
    pub fn stale(&self, session: &SessionId) -> bool {
        let Some(agent) = self.agent(session) else {
            return false;
        };
        self.skills
            .iter()
            .any(|one| one.changed_at > agent.started_at)
    }

    pub fn agent(&self, session: &SessionId) -> Option<&Agent> {
        self.agents
            .iter()
            .find(|(id, _)| id == session)
            .map(|(_, a)| a)
    }

    pub fn terminal(&self, session: &SessionId) -> Option<&Terminal> {
        self.agent(session)?.terminal.as_ref()
    }

    /// Every running agent in these colours.
    pub fn recolor(&self, palette: groove_types::AnsiPalette) {
        let running = self.agents.iter().filter_map(|(_, a)| a.terminal.as_ref());
        running
            .chain(&self.login)
            .for_each(|one| one.recolor(palette));
    }

    pub fn activity(&self, session: &SessionId) -> Option<&SessionActivity> {
        self.agent(session).map(|a| &a.activity)
    }

    /// The launch's result: a running terminal, or an error on the row.
    pub fn started(&mut self, session: SessionId, result: Result<Terminal, Error>, now: Timestamp) {
        let (terminal, status) = match result {
            Ok(terminal) => (Some(terminal), AgentStatus::Idle),
            Err(e) => (None, AgentStatus::Error { message: e.message }),
        };
        let agent = Agent {
            terminal,
            activity: activity(status, now),
            started_at: now,
        };
        self.agents.retain(|(id, _)| id != &session);
        self.agents.push((session, agent));
    }

    /// Removes the agent; the caller ends its terminal.
    pub fn end(&mut self, session: &SessionId) -> Option<Agent> {
        let at = self.agents.iter().position(|(id, _)| id == session)?;
        Some(self.agents.remove(at).1)
    }

    /// One write on the queue, and on the row that asks for it.
    pub fn asked(&mut self, new: NewAsk, reply: Reply) -> Approval {
        let session = new.session.clone();
        let approval = self.asks.queue(new, reply);
        self.told(&session);
        approval
    }

    /// The write one id names, taken off the queue and off the row.
    pub fn resolved(&mut self, id: &ApprovalId) -> Option<(Approval, Reply)> {
        let taken = self.asks.resolve(id)?;
        if let Some(session) = &taken.0.session {
            self.told(session);
        }
        Some(taken)
    }

    /// Every write of a session, taken off the queue for the caller to refuse.
    pub fn forget_asks(&mut self, session: &SessionId) -> Vec<(Approval, Reply)> {
        let dropped = self.asks.forget(session);
        self.told(session);
        dropped
    }

    /// Whether this session's writes run without asking.
    pub fn auto_approve(&mut self, session: &SessionId, on: bool) {
        if let Some(activity) = self.activity_mut(session) {
            activity.auto_approve = on;
        }
    }

    /// The row says what the queue holds.
    fn told(&mut self, session: &SessionId) {
        let asks = self.asks.asks(session, ask_of);
        if let Some(activity) = self.activity_mut(session) {
            activity.asks = asks;
        }
    }

    fn activity_mut(&mut self, session: &SessionId) -> Option<&mut SessionActivity> {
        self.agents
            .iter_mut()
            .find(|(id, _)| id == session)
            .map(|(_, a)| &mut a.activity)
    }
}

pub(crate) fn activity(status: AgentStatus, now: Timestamp) -> SessionActivity {
    SessionActivity {
        status,
        tool: None,
        asks: Vec::new(),
        auto_approve: false,
        changed_at: now,
        seen_at: None,
    }
}

/// The reader thread, the hook receiver and the agent's PTY speak here.
#[derive(Debug)]
pub enum Event {
    /// The terminal's grid changed; the pane redraws.
    Damaged { session: SessionId },
    Exited {
        session: SessionId,
        code: u32,
        at: Timestamp,
    },
    Hook {
        session: SessionId,
        kind: HookKind,
        tool: Option<ToolCall>,
        at: Timestamp,
    },
}

pub fn apply(state: &mut State, event: Event) {
    match event {
        Event::Damaged { .. } => {}
        Event::Exited { session, code, at } => {
            if let Some(activity) = state.activity_mut(&session) {
                activity.status = AgentStatus::Exited { code: code as i32 };
                activity.changed_at = at;
            }
        }
        Event::Hook {
            session,
            kind,
            tool,
            at,
        } => {
            if let Some(activity) = state.activity_mut(&session) {
                hook(activity, kind, tool, at);
            }
        }
    }
}

/// What a hook says the agent is doing. `Notification` moves nothing here.
fn hook(activity: &mut SessionActivity, kind: HookKind, tool: Option<ToolCall>, at: Timestamp) {
    let status = match kind {
        HookKind::SessionStart => AgentStatus::Idle,
        HookKind::UserPromptSubmit | HookKind::PreToolUse | HookKind::PostToolUse => {
            AgentStatus::Working
        }
        HookKind::Stop => AgentStatus::Done { seen: false },
        HookKind::Notification => return,
    };
    activity.tool = match kind {
        HookKind::PreToolUse => tool,
        _ => None,
    };
    if activity.status != status {
        activity.changed_at = at;
    }
    activity.status = status;
}

/// A queued write as the rail and the review sheet say it.
fn ask_of(one: &groove_types::Approval) -> groove_types::Ask {
    groove_types::Ask {
        id: one.id.clone(),
        op: one.op.clone(),
        subject: groove_tools::subject(&one.op, &one.payload),
        text: groove_tools::said(&one.op, &one.payload),
        worktree: groove_tools::acts_in(&one.payload).map(groove_types::WorktreeId::new),
    }
}
