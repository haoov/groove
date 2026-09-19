//! What a press, a drag and a click do, through what the last frame drew.

mod drag;
mod menu;
mod surface;

pub(super) use menu::asked;

use groove_controllers::{AppState, Command, session, task, workspace};
use groove_types::{DiffView, Edit, Motion, SessionId};

use self::drag::{counted, drag_to, grab};
use self::menu::{chosen, lose, palette_row, select_worktree, selector, worktree_menu};
use self::surface::{at, composed, folded, holds, jump, landed, lensed, reached, shown, switch};
use crate::ctx::Metrics;
use crate::hit::{Hits, Target};
use crate::views::session::{Tab, Term};
use crate::{Focus, Surface, Ui};

/// A press on a boundary takes hold of it; anywhere else is a click.
pub(super) fn press(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    ui.clicked = Some(counted(ui.clicked, x, y, metrics));
    if let Some(Target::Split(edge)) = hits.at(x, y) {
        grab(ui, edge, x, y, metrics);
        return Vec::new();
    }
    click(x, y, ui, app, hits, metrics)
}

/// The pointer moved: a boundary follows it, or the open file holds more.
pub(super) fn moved(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    if ui.drag.is_some() {
        drag_to(ui, x, y, metrics);
        return Vec::new();
    }
    if ui.mapping {
        lensed(y, ui, app, hits, metrics);
        return Vec::new();
    }
    if !ui.selecting {
        return Vec::new();
    }
    let Some((path, caret)) = at(ui, app, hits, metrics, (x, y)) else {
        return Vec::new();
    };
    if !holds(app, &path) {
        return Vec::new();
    }
    let edit = Edit::Extend(Motion::To(caret));
    vec![Command::Workspace(workspace::Command::Edit(edit))]
}

/// What was drawn under the point, acted on. Anywhere else closes the palette.
fn click(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let target = hits.at(x, y);
    let inside = matches!(target, Some(Target::Palette | Target::PaletteRow(_)));
    if ui.palette.is_some() && !inside {
        ui.palette = None;
        return Vec::new();
    }
    if ui.menu.is_some() {
        return chosen(target, ui);
    }
    ui.focus = focused(&target, ui.focus);
    acted(target, (x, y), ui, app, hits, metrics)
}

/// What one target does when it is clicked.
fn acted(
    target: Option<Target>,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    match target {
        Some(Target::Board) => board(ui),
        Some(Target::Session(session)) => opened_session(ui, session),
        Some(Target::Tab(tab)) => tabbed(ui, tab),
        Some(Target::Fold) => aside(ui),
        Some(Target::Picker(which)) => selector(ui, app, hits, which),
        Some(Target::Worktree(worktree)) => select_worktree(app, worktree),
        Some(Target::File(path)) => opened(ui, app, path, metrics),
        Some(Target::View(view)) => viewing(ui, app, view, metrics),
        Some(Target::Code) => selecting(point, ui, app, hits, metrics),
        Some(Target::Read(path)) => one(workspace::Command::MarkRead { path }),
        Some(Target::Head(path)) => folded(ui, app, metrics, path),
        Some(Target::Term(term)) => narrowing(ui, term),
        Some(Target::Finding) => finding(ui),
        Some(Target::Found(at)) => reached(ui, app, metrics, at),
        Some(Target::FoundIn(path)) => shut(ui, path),
        Some(Target::Map) => mapping(point.1, ui, app, hits, metrics),
        Some(Target::Stage(path)) => one(workspace::Command::Stage { path }),
        Some(Target::Unstage(path)) => one(workspace::Command::Unstage { path }),
        Some(Target::Discard) => lose(ui),
        Some(Target::Keep) => kept(ui),
        Some(Target::Actions) => actions(ui, hits, metrics),
        Some(Target::Message) => composing(ui, app, hits, metrics, point),
        Some(Target::Do) => acting(ui, app),
        Some(Target::PaletteRow(at)) => palette_row(at, ui, app),
        Some(
            Target::Agent
            | Target::Palette
            | Target::Pinned
            | Target::Split(_)
            | Target::MenuRow(_),
        )
        | None => Vec::new(),
    }
}

/// The board, with a read of the sources behind it.
fn board(ui: &mut Ui) -> Vec<Command> {
    ui.surface = Surface::Board;
    vec![Command::Task(task::Command::Load)]
}

/// A session picked, wherever it was picked from, with the window back on it.
fn opened_session(ui: &mut Ui, session: SessionId) -> Vec<Command> {
    ui.surface = Surface::Session;
    vec![Command::Session(session::Command::Select { session })]
}

fn one(command: workspace::Command) -> Vec<Command> {
    vec![Command::Workspace(command)]
}

fn tabbed(ui: &mut Ui, tab: Tab) -> Vec<Command> {
    ui.session.tab = tab;
    Vec::new()
}

/// The sidebar folded away, or back.
fn aside(ui: &mut Ui) -> Vec<Command> {
    ui.session.folded = !ui.session.folded;
    Vec::new()
}

/// The file's rows shown, with the stream scrolled to where they start.
fn opened(ui: &mut Ui, app: &AppState, path: String, metrics: Metrics) -> Vec<Command> {
    jump(ui, app, &path, metrics);
    shown(app, path)
}

fn viewing(ui: &mut Ui, app: &AppState, view: DiffView, metrics: Metrics) -> Vec<Command> {
    switch(ui, app, view, metrics);
    Vec::new()
}

/// A press in the rows lands the caret and starts a selection.
fn selecting(
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    ui.selecting = true;
    landed(ui, app, hits, metrics, point)
}

/// The keyboard into one term of the sidebar's bar.
fn narrowing(ui: &mut Ui, term: Term) -> Vec<Command> {
    ui.session.composing = false;
    ui.session.bar.focus(term);
    Vec::new()
}

/// The keyboard back into the find bar, at the end of what it holds.
fn finding(ui: &mut Ui) -> Vec<Command> {
    if let Some(find) = ui.session.find.as_mut() {
        find.typing = true;
        find.query.end();
    }
    Vec::new()
}

/// One file's found lines hidden under their own row, or shown again.
fn shut(ui: &mut Ui, path: String) -> Vec<Command> {
    if !ui.session.shut.remove(&path) {
        ui.session.shut.insert(path);
    }
    Vec::new()
}

/// A press on the change column takes hold of the lens.
fn mapping(y: f32, ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) -> Vec<Command> {
    ui.mapping = true;
    lensed(y, ui, app, hits, metrics);
    Vec::new()
}

fn kept(ui: &mut Ui) -> Vec<Command> {
    ui.discarding = None;
    Vec::new()
}

fn actions(ui: &mut Ui, hits: &Hits, metrics: Metrics) -> Vec<Command> {
    ui.menu = Some(worktree_menu(ui, hits, metrics));
    Vec::new()
}

/// The keyboard into the commit box, with its caret where the click landed.
fn composing(
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Vec<Command> {
    ui.session.composing = true;
    composed(app, hits, metrics, point)
}

/// The one action the commit box offers now.
fn acting(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    ui.session.composing = false;
    let act = crate::views::session::commit::primary(app);
    act.map(Command::Workspace).into_iter().collect()
}

/// The pane a click lands in. What it lands on says which.
fn focused(target: &Option<Target>, focus: Focus) -> Focus {
    match target {
        Some(Target::Session(_)) => Focus::Rail,
        Some(Target::Agent) => Focus::Agent,
        Some(Target::Code) | Some(Target::View(_)) => Focus::Workspace,
        Some(
            Target::File(_)
            | Target::Stage(_)
            | Target::Unstage(_)
            | Target::Discard
            | Target::Keep
            | Target::Message
            | Target::Do,
        ) => Focus::Sidebar,
        _ => focus,
    }
}
