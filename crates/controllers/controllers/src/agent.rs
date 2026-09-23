//! The `agent` controller: one function per user action on the `agent` service.

mod pointer;
pub mod skills;

use groove_agent_service::{Event as AgentEvent, LaunchPaths, Select, launch, palette};
use groove_types::{ApprovalId, Session, SessionId, Timestamp};

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

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
    DeleteSkill { name: String },
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
        Command::DeleteSkill { name } => skills::delete(state, spawner, name),
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
    state.agent.auto_approve(session, on);
    crate::session::set_auto_approve(state, services, spawner, session, on);
}

/// The launch runs as a job; its continuation stores the terminal or the error.
/// Every agent runs at the worktree root: the cwd carries no session.
pub fn start(state: &mut AppState, spawner: &dyn Spawner, id: SessionId, size: (u16, u16)) {
    let Some(session) = state
        .session
        .open
        .iter()
        .find(|o| o.session.id == id)
        .map(|o| o.session.clone())
    else {
        return;
    };
    let paths = LaunchPaths {
        home: state.env.home.clone(),
        launch_dir: state.env.data_dir.join("agent-launch"),
        plugin_dirs: groove_agent_service::skills::plugin_dirs(&skills::dirs(state)),
        hooks: state.env.hooks.clone(),
        tools: state.env.tools.clone(),
    };
    let cwd = state.config.worktree_root(&state.env.home);
    let palette = palette(state.config.theme());
    let sink = spawner.sink();
    spawner.spawn(Box::pin(async move {
        let result = launch(
            &session,
            &paths,
            &cwd,
            size,
            palette,
            on_damage(&sink, &session),
            on_exit(&sink, &session),
        );
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.agent.started(session.id, result, Timestamp::now())
        }) as Continuation
    }));
}

pub fn end(state: &mut AppState, session: &SessionId) {
    crate::tools::drop_asks(state, session);
    if let Some(terminal) = state.agent.end(session).and_then(|a| a.terminal) {
        let _ = terminal.terminate();
    }
}

pub fn send(state: &mut AppState, session: &SessionId, bytes: &[u8]) {
    if let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) {
        let _ = terminal.write(bytes);
    }
}

pub fn resize(state: &mut AppState, session: &SessionId, cols: u16, rows: u16) {
    if let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) {
        let _ = terminal.resize(cols, rows);
    }
}

/// One `Damaged` in flight at most, however fast the child writes.
fn on_damage(
    sink: &std::sync::Arc<dyn crate::Deliver>,
    session: &Session,
) -> Box<dyn Fn() + Send + Sync> {
    let id = session.id.clone();
    Box::new(coalesced(sink.clone(), move || {
        let id = id.clone();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(Event::Agent(AgentEvent::Damaged { session: id }), state)
        })
    }))
}

fn on_exit(
    sink: &std::sync::Arc<dyn crate::Deliver>,
    session: &Session,
) -> Box<dyn FnOnce(u32) + Send> {
    let sink = sink.clone();
    let id = session.id.clone();
    Box::new(move |code| {
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                let event = AgentEvent::Exited {
                    session: id,
                    code,
                    at: Timestamp::now(),
                };
                apply(Event::Agent(event), state);
            },
        ));
    })
}
