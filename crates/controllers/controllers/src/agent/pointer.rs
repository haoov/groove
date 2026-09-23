//! What the pointer does to the agent's own screen: a selection, a click the program
//! reads, the wheel, and what is copied out of it.

use groove_agent_service::Select;
use groove_types::SessionId;

use super::Command;
use crate::{AppState, Services};

/// The commands the pointer sends. Anything else is not the pointer's.
pub(super) fn acted(state: &mut AppState, services: &Services, command: Command) {
    match command {
        Command::Select {
            session,
            col,
            row,
            kind,
            from,
        } => select(state, &session, (col, row), kind, from),
        Command::Click {
            session,
            col,
            row,
            down,
        } => click(state, &session, (col, row), down),
        Command::Drag { session, col, row } => drag(state, &session, (col, row)),
        Command::Paste { session, text } => paste(state, &session, &text),
        Command::Copy { session } => copy(state, services, &session),
        Command::Scroll {
            session,
            lines,
            col,
            row,
        } => scroll(state, &session, lines, (col, row)),
        _ => {}
    }
}

/// A selection of the agent's screen, begun at a cell or carried to one.
fn select(
    state: &mut AppState,
    session: &SessionId,
    cell: (usize, usize),
    kind: Select,
    from: bool,
) {
    let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) else {
        return;
    };
    match from {
        true => terminal.select_from(cell, kind),
        false => terminal.select_to(cell),
    }
}

/// The left button on the agent's screen, for the program that reads the mouse.
fn click(state: &mut AppState, session: &SessionId, cell: (usize, usize), down: bool) {
    if let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) {
        let _ = terminal.click(cell, down);
    }
}

/// The pointer moved with the button down, for that same program.
fn drag(state: &mut AppState, session: &SessionId, cell: (usize, usize)) {
    if let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) {
        let _ = terminal.drag(cell);
    }
}

/// The clipboard typed at the program the agent runs.
fn paste(state: &mut AppState, session: &SessionId, text: &str) {
    if let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) {
        let _ = terminal.paste(text);
    }
}

/// What is selected on the agent's screen, onto the clipboard.
fn copy(state: &mut AppState, services: &Services, session: &SessionId) {
    let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) else {
        return;
    };
    if let Some(said) = terminal.selected() {
        let _ = services.clipboard.write(&said);
    }
}

/// The wheel over the agent's screen, at the cell the pointer stands on.
fn scroll(state: &mut AppState, session: &SessionId, lines: i32, cell: (usize, usize)) {
    if let Some(terminal) = state.agent.agent(session).and_then(|a| a.terminal.as_ref()) {
        let _ = terminal.wheel(lines, cell);
    }
}
