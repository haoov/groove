//! The app's own ring: what an action does from any pane.

use groove_controllers::{AppState, Command, session, shell, workspace};

use super::super::Key;
use super::panes::in_rail;
use crate::keymap::Action;
use crate::palette::Palette;
use crate::views::session::Tab;
use crate::{Focus, Grid, Overlay, Surface, Ui};

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
        Action::FocusRail => ui.focus = Focus::Rail,
        Action::FocusAgent => ui.focus = Focus::Agent,
        Action::FocusWorkspace => ui.focus = Focus::Workspace,
        Action::FocusSidebar => sidebar(ui),
        Action::FoldSidebar => ui.session.folded = !ui.session.folded,
        Action::Overview => ui.session.tab = Tab::Overview,
        Action::Diff => ui.session.tab = Tab::Diff,
        Action::Files => ui.session.tab = Tab::Files,
        Action::Resources if holds_clusters(app) => ui.session.tab = Tab::Resources,
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

/// The sidebar unfolded and given the keys, or folded away when it has them.
fn sidebar(ui: &mut Ui) {
    if !ui.session.tab.has_sidebar() {
        return;
    }
    let held = !ui.session.folded && ui.focus == Focus::Sidebar;
    ui.session.folded = held;
    ui.focus = match held {
        true => Focus::Workspace,
        false => Focus::Sidebar,
    };
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

/// The selection of the terminal that has the keys, to the clipboard.
pub(super) fn copied(focus: Focus, app: &AppState) -> Vec<Command> {
    let grid = match focus {
        Focus::Terminal => app
            .session
            .selected
            .as_ref()
            .and_then(|session| app.shell.shells(session)?.focused())
            .map(Grid::Shell),
        _ => Some(Grid::Agent),
    };
    grid.map_or_else(Vec::new, |grid| crate::input::selection::copied(app, grid))
}

fn holds_clusters(app: &AppState) -> bool {
    app.session
        .selected()
        .is_some_and(|open| open.holds_clusters())
}
