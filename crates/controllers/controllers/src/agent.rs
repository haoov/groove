//! The `agent` controller: one function per user action on the `agent` service.

use std::path::PathBuf;

use groove_agent_service::{Event as AgentEvent, LaunchPaths, launch, palette};
use groove_types::{Session, SessionId, Timestamp};

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Event, Spawner, apply};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `agent.start`: launch the session's agent at `cwd` on a grid of `cols` by `rows`.
    Start {
        session: SessionId,
        cwd: PathBuf,
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
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Start { .. } => "agent.start",
            Command::End { .. } => "agent.end",
            Command::Send { .. } => "agent.send",
            Command::Resize { .. } => "agent.resize",
        }
    }
}

pub fn dispatch(command: Command, state: &mut AppState, spawner: &dyn Spawner) {
    match command {
        Command::Start {
            session,
            cwd,
            cols,
            rows,
        } => start(state, spawner, session, cwd, (cols, rows)),
        Command::End { session } => end(state, &session),
        Command::Send { session, bytes } => send(state, &session, &bytes),
        Command::Resize {
            session,
            cols,
            rows,
        } => resize(state, &session, cols, rows),
    }
}

/// The launch runs as a job; its continuation stores the terminal or the error.
pub fn start(
    state: &mut AppState,
    spawner: &dyn Spawner,
    id: SessionId,
    cwd: PathBuf,
    size: (u16, u16),
) {
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
        plugin_dirs: state.env.plugin_dirs.clone(),
    };
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
        Box::new(move |state: &mut AppState| {
            state.agent.started(session.id, result, Timestamp::now())
        }) as Continuation
    }));
}

pub fn end(state: &mut AppState, session: &SessionId) {
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
        Box::new(move |state: &mut AppState| {
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
        sink.deliver(Box::new(move |state: &mut AppState| {
            let event = AgentEvent::Exited {
                session: id,
                code,
                at: Timestamp::now(),
            };
            apply(Event::Agent(event), state);
        }));
    })
}
