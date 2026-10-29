//! The app's own ring: what an action does from any pane.

use groove_controllers::{AppState, Command, cluster, session, shell, workspace};

use super::super::Key;
use super::panes::in_rail;
use crate::hit::Picks;
use crate::keymap::Action;
use crate::palette::{Action as FlowAction, Flow, Palette};
use crate::views::session::Tab;
use crate::views::session::resources::Scoping;
use crate::{Focus, Grid, Overlay, Surface, Ui};

pub(super) fn run(action: Action, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let selected = app.session.selected.clone();
    match action {
        Action::Palette => return palette(ui),
        Action::FindFile => return files(ui),
        Action::SelectNamespace | Action::SelectContext => return scope(ui, app, action),
        Action::SelectKind => return kinds(ui, app),
        Action::SelectWorktree | Action::SelectSession => return flowed(action, ui, app),
        Action::Settings if ui.settings.open => return ui.close_settings(),
        Action::Settings => return ui.open_settings(),
        Action::Board => return board(ui),
        Action::NewExplorer => return new_explorer(),
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
        Action::FocusWorkspace => workspace(ui),
        Action::FocusSidebar => sidebar(ui),
        Action::NextTab | Action::PreviousTab => {
            let forward = action == Action::NextTab;
            if ui.focus == Focus::Terminal {
                return terminal_tab(app, forward);
            }
            ui.session.tab = stepped_tab(ui.session.tab, forward, holds_clusters(app));
        }
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

/// The scope panel asked for, or closed when it stands open.
fn scope(ui: &mut Ui, app: &AppState, action: Action) -> Vec<Command> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let scoping = match action {
        Action::SelectContext => Scoping::new(Picks::Contexts),
        _ => Scoping::asked(open),
    };
    let same = |one: &Overlay| matches!(one, Overlay::Scope(shown) if shown.which == scoping.which);
    if ui.close(same).is_some() {
        return Vec::new();
    }
    ui.surface = Surface::Session;
    let (scoping, commands) = scoping.opened(open);
    ui.overlay = Some(Overlay::Scope(scoping));
    commands
}

/// The palette on the kinds the session's contexts serve, those not yet known asked for.
pub(super) fn kinds(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(open) = app.session.selected().filter(|one| one.holds_clusters()) else {
        return Vec::new();
    };
    ui.surface = Surface::Session;
    let flow = Flow::new(FlowAction::PickKind, open.session.id.clone());
    ui.overlay = Some(Overlay::Palette(Palette {
        flow: Some(flow),
        ..Palette::default()
    }));
    let store = &app.cluster.store;
    let unknown = crate::views::session::resources::contexts(open)
        .into_iter()
        .filter(|one| store.kinds(one).is_none() && !store.discovering(one));
    let discover = |context: &str| cluster::Command::Discover {
        context: context.to_string(),
        again: false,
    };
    unknown.map(|one| Command::Cluster(discover(one))).collect()
}

/// The palette on the session's worktrees, as `repo/branch`, or on the other open sessions.
fn flowed(action: Action, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let open = app.session.selected();
    let flow = match action {
        Action::SelectSession => {
            let session = open.map(|one| one.session.id.clone());
            let session = session.unwrap_or_else(|| groove_types::SessionId::new(""));
            Flow::new(FlowAction::SelectSession, session)
        }
        _ => match open.filter(|one| !one.worktrees.is_empty()) {
            Some(open) => Flow::new(FlowAction::SelectWorktree, open.session.id.clone()),
            None => return Vec::new(),
        },
    };
    ui.surface = Surface::Session;
    ui.overlay = Some(Overlay::Palette(Palette {
        flow: Some(flow),
        ..Palette::default()
    }));
    Vec::new()
}

fn new_explorer() -> Vec<Command> {
    let open = session::Command::OpenExplorer { title: None };
    vec![Command::Session(open)]
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

/// The workspace unfolded, then given the keys, then folded away under the agent.
fn workspace(ui: &mut Ui) {
    let held = !ui.session.stowed && ui.focus == Focus::Workspace;
    ui.session.stowed = held;
    ui.focus = match held {
        true => Focus::Agent,
        false => Focus::Workspace,
    };
}

/// The palette on the selected worktree's files.
fn files(ui: &mut Ui) -> Vec<Command> {
    let mut palette = Palette::default();
    palette.query.set("#");
    ui.overlay = Some(Overlay::Palette(palette));
    vec![Command::Workspace(workspace::Command::ListPaths)]
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
    ui.focus = match (held, ui.session.stowed) {
        (true, true) => Focus::Agent,
        (true, false) => Focus::Workspace,
        (false, _) => Focus::Terminal,
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
pub(super) fn terminal_tab(app: &AppState, forward: bool) -> Vec<Command> {
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
