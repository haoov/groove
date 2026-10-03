//! The app's own ring: what an action does from any pane.

use groove_controllers::{AppState, Command, agent, session, shell, workspace};

use super::super::Key;
use super::panes::in_rail;
use crate::keymap::Action;
use crate::palette::Palette;
use crate::views::session::Tab;
use crate::{Focus, Overlay, Surface, Ui};

pub(super) fn run(action: Action, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let selected = app.session.selected.clone();
    match action {
        Action::Palette => return palette(ui),
        Action::Settings if ui.settings.open => return ui.close_settings(),
        Action::Settings => return ui.open_settings(),
        Action::Board => return board(ui),
        Action::NewExplorer => {
            let open = session::Command::OpenExplorer { title: None };
            return vec![Command::Session(open)];
        }
        Action::CloseSession => {
            let close = selected.map(|session| session::Command::Close { session });
            return close.map(Command::Session).into_iter().collect();
        }
        Action::NextSession => return in_rail(Key::Down, ui, app),
        Action::PreviousSession => return in_rail(Key::Up, ui, app),
        Action::FocusLeft | Action::FocusRight => {
            ui.focus = ui.focus.beside(action == Action::FocusRight);
        }
        Action::FoldSidebar => ui.session.folded = !ui.session.folded,
        Action::Overview => ui.session.tab = Tab::Overview,
        Action::Diff => ui.session.tab = Tab::Diff,
        Action::Files => ui.session.tab = Tab::Files,
        Action::Terminals => terminals(ui),
        Action::NewTerminal => return new_terminal(ui, selected),
        Action::Reload => return vec![Command::Workspace(workspace::Command::Load)],
        _ => {}
    }
    Vec::new()
}

/// The palette, or closed when it stands open.
fn palette(ui: &mut Ui) -> Vec<Command> {
    if ui.close(|one| matches!(one, Overlay::Palette(_))).is_some() {
        return Vec::new();
    }
    ui.overlay = Some(Overlay::Palette(Palette::default()));
    vec![Command::Session(session::Command::ListRepos)]
}

/// The board, or the session it was opened from.
fn board(ui: &mut Ui) -> Vec<Command> {
    ui.surface = match ui.surface {
        Surface::Board => Surface::Session,
        Surface::Session => Surface::Board,
    };
    match ui.surface {
        Surface::Board => crate::input::pointer::board_reads(),
        Surface::Session => Vec::new(),
    }
}

/// The terminals up and given the keys, or folded away when they have them.
fn terminals(ui: &mut Ui) {
    let held = ui.session.manual && ui.focus == Focus::Terminal;
    ui.session.manual = !held;
    ui.focus = match held {
        true => Focus::Workspace,
        false => Focus::Terminal,
    };
}

fn new_terminal(ui: &mut Ui, selected: Option<groove_types::SessionId>) -> Vec<Command> {
    let Some(session) = selected else {
        return Vec::new();
    };
    ui.session.manual = true;
    ui.focus = Focus::Terminal;
    let (cols, rows) = groove_controllers::session::FIRST_SIZE;
    vec![Command::Shell(shell::Command::Open {
        session,
        cols,
        rows,
    })]
}

/// The agent's selection to the clipboard, from its own terminal.
pub(super) fn copied(app: &AppState) -> Vec<Command> {
    let copy = app
        .session
        .selected
        .clone()
        .map(|session| agent::Command::Copy { session });
    copy.map(Command::Agent).into_iter().collect()
}
