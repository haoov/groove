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
        Action::FocusRail | Action::FocusAgent | Action::FocusWorkspace | Action::FocusSidebar
            if ui.showing(app) == Surface::Board =>
        {
            on_board(action, ui);
        }
        Action::FocusRail => ui.focus = Focus::Rail,
        Action::FocusAgent => ui.focus = Focus::Agent,
        Action::FocusWorkspace => ui.focus = Focus::Workspace,
        Action::FocusSidebar => sidebar(ui),
        Action::FocusCommit => commit_box(ui),
        Action::NextTab | Action::PreviousTab => {
            let forward = action == Action::NextTab;
            if ui.focus == Focus::Terminal {
                return terminal_tab(app, forward);
            }
            ui.session.tab = stepped_tab(ui.session.tab, forward, holds_clusters(app));
        }
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

/// On the board, `alt+1` is the rail and the next three are its columns.
fn on_board(action: Action, ui: &mut Ui) {
    use crate::views::board::List;
    let list = match action {
        Action::FocusAgent => Some(List::Live),
        Action::FocusWorkspace => Some(List::Next),
        Action::FocusSidebar => Some(List::Review),
        _ => None,
    };
    ui.board.list = list;
    ui.focus = match list {
        Some(_) => Focus::Workspace,
        None => Focus::Rail,
    };
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
    let held = !ui.session.folded && ui.focus == Focus::Sidebar && !ui.session.composing;
    ui.session.folded = held;
    ui.session.composing = false;
    ui.focus = match held {
        true => Focus::Workspace,
        false => Focus::Sidebar,
    };
}

/// The caret in the commit box, the sidebar unfolded for it.
fn commit_box(ui: &mut Ui) {
    if !ui.session.tab.has_sidebar() {
        return;
    }
    ui.session.folded = false;
    if ui.session.commits() {
        ui.session.composing = true;
        ui.focus = Focus::Sidebar;
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

/// The workspace tab beside `tab`, from the last back to the first.
fn stepped_tab(tab: Tab, forward: bool, clusters: bool) -> Tab {
    let up: Vec<Tab> = Tab::ALL
        .into_iter()
        .filter(|one| *one != Tab::Resources || clusters)
        .collect();
    let at = up.iter().position(|one| *one == tab).unwrap_or(0);
    let len = up.len();
    match forward {
        true => up[(at + 1) % len],
        false => up[(at + len - 1) % len],
    }
}

/// The selected session's next or previous terminal tab, from the last back to the first.
fn terminal_tab(app: &AppState, forward: bool) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let Some(shells) = app.shell.shells(&session) else {
        return Vec::new();
    };
    let len = shells.tabs.len();
    let at = shells
        .tabs
        .iter()
        .position(|tab| Some(tab.id) == shells.selected)
        .unwrap_or(0);
    let next = match forward {
        true => (at + 1) % len.max(1),
        false => (at + len.max(1) - 1) % len.max(1),
    };
    let Some(tab) = shells.tabs.get(next) else {
        return Vec::new();
    };
    vec![Command::Shell(shell::Command::SelectTab {
        session,
        tab: tab.id,
    })]
}
