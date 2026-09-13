//! The agent capability. One agent per open session: its terminal and its activity.

mod launch;

use groove_types::{AgentStatus, Error, HookKind, SessionActivity, SessionId, Timestamp, ToolCall};

pub use groove_terminal::Terminal;
pub use groove_types::Screen;
pub use launch::{LaunchPaths, launch, palette};

/// One session's agent. `terminal` is `None` when the launch failed.
#[derive(Debug)]
pub struct Agent {
    pub terminal: Option<Terminal>,
    pub activity: SessionActivity,
}

/// The `agent` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub agents: Vec<(SessionId, Agent)>,
}

impl State {
    pub fn agent(&self, session: &SessionId) -> Option<&Agent> {
        self.agents
            .iter()
            .find(|(id, _)| id == session)
            .map(|(_, a)| a)
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
        };
        self.agents.retain(|(id, _)| id != &session);
        self.agents.push((session, agent));
    }

    /// Removes the agent; the caller ends its terminal.
    pub fn end(&mut self, session: &SessionId) -> Option<Agent> {
        let at = self.agents.iter().position(|(id, _)| id == session)?;
        Some(self.agents.remove(at).1)
    }

    fn activity_mut(&mut self, session: &SessionId) -> Option<&mut SessionActivity> {
        self.agents
            .iter_mut()
            .find(|(id, _)| id == session)
            .map(|(_, a)| &mut a.activity)
    }
}

fn activity(status: AgentStatus, now: Timestamp) -> SessionActivity {
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
        Event::Hook { .. } => {}
    }
}
