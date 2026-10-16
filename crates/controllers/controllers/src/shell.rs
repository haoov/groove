//! The `shell` controller: one function per user action on a session's own terminals.

use groove_agent_service::Select;
use groove_shell_service::{Event as ShellEvent, Hooks, spawn, spec};
use groove_types::SessionId;

use crate::spawn::coalesced;
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `shell.open`: a login shell in the session's directory, in a tab of its own.
    Open {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// `shell.split`: another shell beside the ones of the tab that is up.
    Split {
        session: SessionId,
        cols: u16,
        rows: u16,
    },
    /// `shell.close`: one terminal ended and taken away; its tab goes when it was the last.
    Close { session: SessionId, id: u64 },
    /// `shell.close_tab`: a tab and every terminal in it.
    CloseTab { session: SessionId, tab: u64 },
    /// `shell.select_tab`: which tab the section shows.
    SelectTab { session: SessionId, tab: u64 },
    /// `shell.focus`: the terminal the keys go to, its tab brought up.
    Focus { session: SessionId, id: u64 },
    /// `shell.send`: bytes to the terminal, as typed.
    Send {
        session: SessionId,
        id: u64,
        bytes: Vec<u8>,
    },
    /// `shell.paste`: text typed at the terminal, bracketed when it asked.
    Paste {
        session: SessionId,
        id: u64,
        text: String,
    },
    /// `shell.resize`: the section's grid to the terminal.
    Resize {
        session: SessionId,
        id: u64,
        cols: u16,
        rows: u16,
    },
    /// `shell.scroll`: the wheel over a terminal, at the cell it stands on.
    Scroll {
        session: SessionId,
        id: u64,
        lines: i32,
        col: usize,
        row: usize,
    },
    /// `shell.select`: a selection of a terminal's grid, begun at a cell or carried to one.
    Select {
        session: SessionId,
        id: u64,
        col: usize,
        row: usize,
        kind: Select,
        from: bool,
    },
    /// `shell.click`: the left button on a terminal, for the program that reads it.
    Click {
        session: SessionId,
        id: u64,
        col: usize,
        row: usize,
        down: bool,
    },
    /// `shell.drag`: the pointer moved with the button down, for that same program.
    Drag {
        session: SessionId,
        id: u64,
        col: usize,
        row: usize,
    },
    /// `shell.copy`: what is selected on a terminal's grid, to the clipboard.
    Copy { session: SessionId, id: u64 },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Open { .. } => "shell.open",
            Command::Split { .. } => "shell.split",
            Command::Close { .. } => "shell.close",
            Command::CloseTab { .. } => "shell.close_tab",
            Command::SelectTab { .. } => "shell.select_tab",
            Command::Focus { .. } => "shell.focus",
            Command::Send { .. } => "shell.send",
            Command::Paste { .. } => "shell.paste",
            Command::Resize { .. } => "shell.resize",
            Command::Scroll { .. } => "shell.scroll",
            Command::Select { .. } => "shell.select",
            Command::Click { .. } => "shell.click",
            Command::Drag { .. } => "shell.drag",
            Command::Copy { .. } => "shell.copy",
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
        Command::Open {
            session,
            cols,
            rows,
        } => open(state, services, spawner, (session, false), (cols, rows)),
        Command::Split {
            session,
            cols,
            rows,
        } => open(state, services, spawner, (session, true), (cols, rows)),
        Command::Close { session, id } => close(state, &session, id),
        Command::CloseTab { session, tab } => ended(state.shell.close_tab(&session, tab)),
        Command::SelectTab { session, tab } => state.shell.select_tab(&session, tab),
        Command::Focus { session, id } => state.shell.focus(&session, id),
        Command::Copy { session, id } => {
            if let Some(terminal) = state.shell.terminal(&session, id) {
                crate::grid::copy(terminal, services, spawner);
            }
        }
        typed => to_terminal(state, typed),
    }
}

/// What goes straight to a running terminal: keys, a paste, its grid, the wheel.
fn to_terminal(state: &AppState, command: Command) {
    let (session, id) = match &command {
        Command::Send { session, id, .. }
        | Command::Paste { session, id, .. }
        | Command::Resize { session, id, .. }
        | Command::Scroll { session, id, .. }
        | Command::Select { session, id, .. }
        | Command::Click { session, id, .. }
        | Command::Drag { session, id, .. } => (session, *id),
        _ => return,
    };
    let Some(terminal) = state.shell.terminal(session, id) else {
        return;
    };
    let _ = match &command {
        Command::Send { bytes, .. } => terminal.write(bytes),
        Command::Paste { text, .. } => terminal.paste(text),
        Command::Resize { cols, rows, .. } => terminal.resize(*cols, *rows),
        Command::Scroll {
            lines, col, row, ..
        } => terminal.wheel(*lines, (*col, *row)),
        Command::Click { col, row, down, .. } => terminal.click((*col, *row), *down),
        Command::Drag { col, row, .. } => terminal.drag((*col, *row)),
        Command::Select {
            col,
            row,
            kind,
            from,
            ..
        } => {
            crate::grid::select(terminal, (*col, *row), *kind, *from);
            Ok(())
        }
        _ => Ok(()),
    };
}

/// A new terminal's place at once, alone or `beside` the tab's; the shell spawned in a job.
fn open(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (session, beside): (SessionId, bool),
    size: (u16, u16),
) {
    let id = state.shell.reserve(&session, beside);
    let palette = groove_agent_service::palette(state.config.theme());
    let (program, home) = (state.env.shell.clone(), state.env.home.clone());
    let (sink, service) = (spawner.sink(), services.session.clone());
    spawner.spawn(Box::pin(async move {
        let hooks = hooks(&sink, &session, id);
        let result = match service.session_dir(&session).await {
            Ok(cwd) => Ok(spawn(spec(&program, (cwd, &home), size), palette, hooks).await),
            Err(e) => Err(e),
        };
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match result {
                Ok(result) => state.shell.started(&session, id, result),
                Err(e) => {
                    state.shell.close(&session, id);
                    state.failed(e);
                }
            },
        ) as Continuation
    }));
}

pub fn close(state: &mut AppState, session: &SessionId, id: u64) {
    ended(state.shell.close(session, id).into_iter().collect());
}

/// Every terminal of a session ended, when the session goes.
pub fn end(state: &mut AppState, session: &SessionId) {
    ended(state.shell.end(session));
}

fn ended(shells: Vec<groove_shell_service::Shell>) {
    for terminal in shells.into_iter().filter_map(|shell| shell.terminal) {
        let _ = terminal.terminate();
    }
}

fn hooks(sink: &std::sync::Arc<dyn crate::Deliver>, session: &SessionId, id: u64) -> Hooks {
    let damaged = session.clone();
    let on_damage = Box::new(coalesced(sink.clone(), move || {
        let session = damaged.clone();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(Event::Shell(ShellEvent::Damaged { session, id }), state)
        })
    }));
    let (sink, session) = (sink.clone(), session.clone());
    let on_exit = Box::new(move |code| {
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                apply(
                    Event::Shell(ShellEvent::Exited { session, id, code }),
                    state,
                )
            },
        ));
    });
    Hooks { on_damage, on_exit }
}
