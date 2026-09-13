//! The agent capability. Its slice of `AppState`, the operations on it, its events.

use groove_types::{HookKind, SessionActivity, SessionId, ToolCall};

/// One `SessionActivity` per open session.
#[derive(Debug, Default)]
pub struct State {
    pub activity: Vec<(SessionId, SessionActivity)>,
}

impl State {
    pub fn activity(&self, session: &SessionId) -> Option<&SessionActivity> {
        self.activity
            .iter()
            .find(|(id, _)| id == session)
            .map(|(_, a)| a)
    }
}

/// The hook receiver and the agent's PTY speak here.
#[derive(Debug)]
pub enum Event {
    Hook {
        session: SessionId,
        kind: HookKind,
        tool: Option<ToolCall>,
    },
    Exited {
        session: SessionId,
        code: i32,
    },
}

pub fn apply(_state: &mut State, event: Event) {
    match event {
        Event::Hook { .. } | Event::Exited { .. } => {}
    }
}
