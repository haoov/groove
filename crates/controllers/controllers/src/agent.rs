//! The `agent` controller: one function per user action on the `agent` service.

mod pointer;
pub mod skills;
mod start;

use groove_agent_service::Select;
use groove_types::{ApprovalId, SessionId};

use crate::{AppState, Services, Spawner};

pub use start::start;
pub(crate) use start::{asking, launch_dir};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `agent.start`: launch the session's agent at the worktree root on a grid of `cols` by `rows`.
    Start {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// `agent.end`: end the agent and forget it.
    End { session: SessionId },
    /// `agent.send`: bytes to the PTY, as typed.
    Send { session: SessionId, bytes: Vec<u8> },
    /// `agent.resize`: the pane's grid to the PTY.
    Resize {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// `agent.approve`: the write the agent asked for runs.
    Approve { id: ApprovalId },
    /// `agent.refuse`: it does not, and its agent hears so.
    Refuse { id: ApprovalId },
    /// `agent.auto_approve`: every write of this session runs without asking.
    AutoApprove { session: SessionId, on: bool },
    /// `agent.select`: a selection of the agent's screen, begun or carried on.
    Select {
        session: SessionId,
        col: usize,
        row: usize,
        kind: Select,
        from: bool,
    },
    /// `agent.click`: the left button on the screen, for the program that reads it.
    Click {
        session: SessionId,
        col: usize,
        row: usize,
        down: bool,
    },
    /// `agent.drag`: the pointer moved with the button down, for that same program.
    Drag {
        session: SessionId,
        col: usize,
        row: usize,
    },
    /// `agent.paste`: the clipboard typed at the agent, bracketed when it asked.
    Paste { session: SessionId, text: String },
    /// `agent.copy`: what is selected on the agent's screen, to the clipboard.
    Copy { session: SessionId },
    /// `agent.scroll`: the wheel over the agent's own screen, at the cell it stands on.
    Scroll {
        session: SessionId,
        lines: i32,
        col: usize,
        row: usize,
    },
    /// `agent.reload`: the agent ended and started again, resuming its own thread.
    Reload {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// `agent.list_skills`: both plugins written, then what they offer.
    ListSkills,
    /// `agent.send_skill`: one skill typed into the agent's own prompt.
    SendSkill {
        session: SessionId,
        id: String,
        args: Option<String>,
    },
    /// `agent.delete_skill`: one skill of the user's own.
    DeleteSkill { id: String },
    /// `agent.switch_skill`: one skill given to sessions, or no longer; a core one stays on.
    SwitchSkill { id: String, on: bool },
    /// `agent.run_routine`: one routine run now, by its button.
    RunRoutine { id: String },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Start { .. } => "agent.start",
            Command::End { .. } => "agent.end",
            Command::Send { .. } => "agent.send",
            Command::Resize { .. } => "agent.resize",
            Command::Approve { .. } => "agent.approve",
            Command::Refuse { .. } => "agent.refuse",
            Command::AutoApprove { .. } => "agent.auto_approve",
            Command::Select { .. } => "agent.select",
            Command::Click { .. } => "agent.click",
            Command::Drag { .. } => "agent.drag",
            Command::Paste { .. } => "agent.paste",
            Command::Copy { .. } => "agent.copy",
            Command::Scroll { .. } => "agent.scroll",
            Command::Reload { .. } => "agent.reload",
            Command::ListSkills => "agent.list_skills",
            Command::SendSkill { .. } => "agent.send_skill",
            Command::DeleteSkill { .. } => "agent.delete_skill",
            Command::SwitchSkill { .. } => "agent.switch_skill",
            Command::RunRoutine { .. } => "agent.run_routine",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Start {
            session,
            cols,
            rows,
        } => start(state, spawner, session, (cols, rows)),
        Command::End { session } => end(state, &session),
        Command::Send { session, bytes } => send(state, &session, &bytes),
        Command::Resize {
            session,
            cols,
            rows,
        } => resize(state, &session, cols, rows),
        Command::Approve { id } => crate::tools::allow(state, services, spawner, &id),
        Command::Refuse { id } => crate::tools::refuse(state, &id),
        Command::AutoApprove { session, on } => {
            auto_approve(state, services, spawner, &session, on)
        }
        Command::Reload {
            session,
            cols,
            rows,
        } => reload(state, spawner, session, (cols, rows)),
        Command::ListSkills => skills::list(state, spawner),
        Command::SendSkill { session, id, args } => {
            skills::send(state, spawner, &session, &id, args.as_deref())
        }
        Command::DeleteSkill { id } => skills::delete(state, spawner, &id),
        Command::SwitchSkill { id, on } => skills::switch(state, spawner, id, on),
        Command::RunRoutine { id } => crate::routine::button(state, services, spawner, &id),
        pointing => pointer::acted(state, services, pointing),
    }
}

/// The agent ended and started again. The launch resumes its own thread by itself.
fn reload(state: &mut AppState, spawner: &dyn Spawner, session: SessionId, size: (u16, u16)) {
    end(state, &session);
    start(state, spawner, session, size);
}

/// Every write of this session runs without asking, or waits again.
fn auto_approve(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    on: bool,
) {
    if let Some(open) = state.session.get_mut(session) {
        open.state.auto_approve = on;
    }
    crate::session::set_auto_approve(services, spawner, session, on);
}

pub(crate) fn forget(state: &mut AppState, session: &SessionId) {
    if let Err(e) = groove_agent_service::forget(&launch_dir(state), session.as_str()) {
        state.failed(e);
    }
}

pub fn end(state: &mut AppState, session: &SessionId) {
    crate::tools::drop_asks(state, session);
    if let Some(terminal) = state.agent.end(session).and_then(|a| a.terminal) {
        let _ = terminal.terminate();
    }
}

pub fn send(state: &mut AppState, session: &SessionId, bytes: &[u8]) {
    if let Some(terminal) = state.agent.terminal(session) {
        let _ = terminal.write(bytes);
    }
}

pub fn resize(state: &mut AppState, session: &SessionId, cols: u16, rows: u16) {
    if let Some(terminal) = state.agent.terminal(session) {
        let _ = terminal.resize(cols, rows);
    }
}
