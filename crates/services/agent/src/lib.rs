//! The agent capability: each session's agent and its asks.

pub(crate) mod launch;
mod listed;

#[cfg(test)]
mod tests;

use groove_types::{
    AgentStatus, Approval, ApprovalId, Error, HookKind, SessionActivity, SessionId, Timestamp,
    ToolCall,
};

pub use groove_agent_launch::{forget, fresh, hand_over};
pub use groove_approvals::{New as NewAsk, Queue};
pub use groove_hooks::{Post, Receiver};
pub use groove_mcp::{Answer, Call, Reply, Server};
pub use groove_terminal::{Hooks, PtySpec, Select, Terminal};
pub use groove_tools as tools;
pub use groove_types::Screen;

/// The tool Claude calls to put a question to the user.
const ASKS_THE_USER: &str = "AskUserQuestion";
pub use launch::{LaunchPaths, claude_bin, launch, login, palette};

/// One session's agent. `terminal` is `None` when the launch failed.
#[derive(Debug)]
pub struct Agent {
    pub terminal: Option<Terminal>,
    pub activity: SessionActivity,
    /// When this agent was launched, which says whether a skill is newer than it.
    pub started_at: Timestamp,
    /// Which launch of its session it is, as `State::launch` numbered it.
    pub launch: u64,
}

/// The `agent` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub agents: Vec<(SessionId, Agent)>,
    /// The launches numbered so far.
    launches: u64,
    /// The launch each session waits on.
    pending: Vec<(SessionId, u64)>,
    /// The writes waiting on the user, each holding the answer it owes its agent.
    asks: Queue<Reply>,
    /// The sign-in Setup runs, while it runs.
    pub login: Option<Terminal>,
}

impl State {
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

    /// The number the next launch of `session` goes by, which its exit carries; the one it waits on.
    pub fn launch(&mut self, session: &SessionId) -> u64 {
        self.launches += 1;
        self.pending.retain(|(id, _)| id != session);
        self.pending.push((session.clone(), self.launches));
        self.launches
    }

    /// The launch's result, on the row; a terminal no row takes, or one it displaces, comes back to end.
    pub fn started(
        &mut self,
        (session, launch): (SessionId, u64),
        result: Result<Terminal, Error>,
        now: Timestamp,
    ) -> Option<Terminal> {
        let awaited = self
            .pending
            .iter()
            .any(|one| *one == (session.clone(), launch));
        if !awaited {
            return result.ok();
        }
        self.pending.retain(|(id, _)| id != &session);
        let (terminal, status) = match result {
            Ok(terminal) => (Some(terminal), AgentStatus::Idle),
            Err(e) => (None, AgentStatus::Error { message: e.message }),
        };
        let agent = Agent {
            terminal,
            activity: activity(status, now),
            started_at: now,
            launch,
        };
        let displaced = self.end(&session).and_then(|one| one.terminal);
        self.agents.push((session, agent));
        displaced
    }

    /// Removes the agent and any launch it waits on; the caller ends its terminal.
    pub fn end(&mut self, session: &SessionId) -> Option<Agent> {
        self.pending.retain(|(id, _)| id != session);
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
        let session = taken.0.session.clone();
        self.told(&session);
        Some(taken)
    }

    /// Every write of a session, taken off the queue for the caller to refuse.
    pub fn forget_asks(&mut self, session: &SessionId) -> Vec<(Approval, Reply)> {
        let dropped = self.asks.forget(session);
        self.told(session);
        dropped
    }

    /// The user looked at the session: a finished turn is no longer news.
    pub fn saw(&mut self, session: &SessionId, now: Timestamp) {
        let Some(activity) = self.activity_mut(session) else {
            return;
        };
        if activity.status == (AgentStatus::Done { seen: false }) {
            activity.status = AgentStatus::Done { seen: true };
        }
        activity.seen_at = Some(now);
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
        changed_at: now,
        seen_at: None,
    }
}

/// The reader thread, the hook receiver and the agent's PTY speak here.
#[derive(Debug)]
pub enum Event {
    /// The terminal's grid changed; the pane redraws.
    Damaged { session: SessionId },
    /// The process of one launch ended; a launch since then is not its to mark.
    Exited {
        session: SessionId,
        launch: u64,
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
        Event::Exited {
            session,
            launch,
            code,
            at,
        } => {
            let current = state
                .agent(&session)
                .is_some_and(|one| one.launch == launch);
            if let Some(activity) = state.activity_mut(&session).filter(|_| current) {
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

/// What a hook says the agent is doing; a question or a prompt waits on the user.
fn hook(activity: &mut SessionActivity, kind: HookKind, tool: Option<ToolCall>, at: Timestamp) {
    let asks = tool.as_ref().is_some_and(|one| one.name == ASKS_THE_USER);
    let status = match kind {
        HookKind::SessionStart => AgentStatus::Idle,
        HookKind::PreToolUse if asks => AgentStatus::Asking,
        HookKind::UserPromptSubmit | HookKind::PreToolUse | HookKind::PostToolUse => {
            AgentStatus::Working
        }
        HookKind::Stop => AgentStatus::Done { seen: false },
        HookKind::Notification => AgentStatus::Asking,
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
