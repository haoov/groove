//! What a press, a drag and a click do, through what the last frame drew.

mod agent;
mod board;
mod clusters;
mod drag;
mod focus;
mod header;
mod menu;
mod rail;
mod resources;
mod settings;
mod shell;
mod sidebar;
mod surface;

pub(super) use menu::asked;
pub(super) use surface::middle;

use groove_controllers::{AppState, Command, delivery, task, workspace};
use groove_types::{DiffView, Edit, Motion};

pub(super) use self::board::dropped;
pub(super) use self::board::reads as board_reads;
use self::board::{carried, opened_session, review, takes};
use self::drag::{counted, drag_to, grab};
use self::focus::focused;
use self::header::{finishing, task_menu};
use self::menu::{chosen, lose, palette_row, select_worktree, selector, worktree_menu};
use self::sidebar::{finding, narrowing, note_at, paned, twisty};
use self::surface::{
    at, blamed, closing, composed, folded, holds, in_files, jump, landed, lensed, reached, switch,
    unfolded,
};
use crate::hit::{Hits, Target};
use crate::views::session::Tab;
use crate::{Held, Overlay, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// A press on a boundary takes hold of it; anywhere else is a click.
pub(super) fn press(
    x: f32,
    y: f32,
    mods: super::Modifiers,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    ui.clicked = Some(counted(ui.clicked, x, y, metrics));
    ui.agent.bypassed = mods.shift;
    match hits.at(x, y) {
        Some(Target::Split(edge)) => grab(ui, edge, x, y, metrics),
        Some(Target::Place(id)) => takes(ui, id),
        Some(Target::ResourceEdge(label)) => resources::held(ui, app, hits, (label, x)),
        _ => click(x, y, ui, app, hits, metrics),
    }
}

pub(crate) use agent::{copied as agent_copied, released as agent_released};

/// The pointer moved: a boundary follows it, or the open file holds more.
pub(super) fn moved(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    match &ui.held {
        Some(Held::Edge(_)) => drag_to(ui, x, y, metrics),
        Some(Held::Task(_)) => carried(x, y, ui, app, metrics),
        Some(Held::Lens) => lensed(y, ui, app, hits, metrics),
        Some(Held::AgentText | Held::AgentClick) => {
            return agent::dragged(ui, app, (x, y), metrics);
        }
        Some(Held::Text) => return extended(x, y, ui, app, hits, metrics),
        Some(Held::Column(_)) => resources::dragged(ui, x, metrics),
        None => {}
    }
    Vec::new()
}

/// The pointer moved while it holds text of the open file: the selection carried to it.
fn extended(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
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
    if ui.palette().is_some() && !inside {
        ui.overlay = None;
        return Vec::new();
    }
    if ui.menu().is_some() {
        return chosen(target, ui);
    }
    focused(&target, ui);
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
    if let Some(commands) = elsewhere(target.as_ref(), point, ui, app, hits, metrics) {
        return commands;
    }
    match target {
        Some(Target::Tab(tab)) => tabbed(ui, tab),
        Some(Target::Fold) => aside(ui),
        Some(Target::Picker(which)) => selector(ui, app, hits, which),
        Some(Target::Worktree(worktree)) => select_worktree(app, worktree),
        Some(Target::File(path)) => opened(ui, app, path, metrics),
        Some(Target::View(view)) => viewing(ui, app, view, (hits, metrics)),
        Some(Target::Mode(mode)) => vec![Command::Workspace(workspace::Command::SetMode { mode })],
        Some(Target::Code) => selecting(point, ui, app, hits, metrics),
        Some(Target::Read(path)) => one(workspace::Command::MarkRead { path }),
        Some(Target::Note(origin, button)) => surface::noted(ui, app, origin, button),
        Some(Target::Head(path)) => folded(ui, app, metrics, path),
        Some(Target::Gap { row, way }) => one(workspace::Command::OpenGap { row, way }),
        Some(Target::Term(term)) => narrowing(ui, term),
        Some(Target::Finding) => finding(ui),
        Some(Target::Found(at)) => reached(ui, app, (hits, metrics), at),
        Some(Target::FoundIn(path)) => shut(ui, path),
        Some(Target::Map) => mapping(point.1, ui, app, hits, metrics),
        Some(Target::Actions) => actions(ui, app, hits, metrics),
        Some(Target::Message) => composing(ui, app, hits, metrics, point),
        Some(Target::Do) => acting(ui, app),
        Some(Target::PaletteRow(at)) => palette_row(at, ui, app),
        Some(Target::LogHours(id)) => logging(id),
        Some(Target::Finish(session)) => finishing(session),
        Some(Target::Refresh) => vec![Command::Delivery(delivery::Command::RefreshMr)],
        Some(Target::MrPage(url)) => vec![Command::Delivery(delivery::Command::BrowseMr { url })],
        Some(Target::TaskPage(url) | Target::Link(url)) => one_task(task::Command::Browse { url }),
        Some(Target::Pane(pane)) => paned(ui, pane),
        Some(Target::NoteAt(at)) => note_at(ui, app, metrics, at),
        Some(Target::Commit(sha)) => one(workspace::Command::OpenCommit { sha }),
        Some(Target::Blamed(sha)) => blamed(ui, sha),
        Some(Target::Working) => one(workspace::Command::LeaveCommit),
        Some(target @ (Target::Dir(_) | Target::Group(_))) => twisty(ui, target),
        Some(Target::Review(project, iid)) => review(project, iid),
        Some(Target::TaskActions(session)) => task_menu(ui, hits, session),
        one => staging(one, ui, app),
    }
}

/// What the change itself is asked to keep or give up.
fn staging(target: Option<Target>, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    match target {
        Some(Target::OpenTab(path)) => in_files(ui, app, path),
        Some(Target::CloseTab(path)) => closing(ui, app, path),
        Some(Target::Stage(path)) => one(workspace::Command::Stage { path }),
        Some(Target::Unstage(path)) => one(workspace::Command::Unstage { path }),
        Some(Target::Discard) => lose(ui),
        Some(Target::Keep) => kept(ui),
        _ => Vec::new(),
    }
}

/// The hours the clock measured, handed to the source.
fn logging(external_id: groove_types::ExternalId) -> Vec<Command> {
    let log = groove_controllers::task::Command::LogHours { external_id };
    vec![Command::Task(log)]
}

fn one(command: workspace::Command) -> Vec<Command> {
    vec![Command::Workspace(command)]
}

fn one_task(command: task::Command) -> Vec<Command> {
    vec![Command::Task(command)]
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

/// In the diff, the stream at the file's rows; anything else, the file in the Files tab.
fn opened(ui: &mut Ui, app: &AppState, path: String, metrics: Metrics) -> Vec<Command> {
    if ui.session.tab == Tab::Diff && jump(ui, app, &path, metrics) {
        return unfolded(app, path);
    }
    in_files(ui, app, path)
}

fn viewing(ui: &mut Ui, app: &AppState, view: DiffView, at: (&Hits, Metrics)) -> Vec<Command> {
    switch(ui, app, view, at);
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
    ui.held = Some(Held::Text);
    landed(ui, app, hits, metrics, point)
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
    ui.held = Some(Held::Lens);
    lensed(y, ui, app, hits, metrics);
    Vec::new()
}

fn kept(ui: &mut Ui) -> Vec<Command> {
    ui.close(|one| matches!(one, Overlay::Losing(_)));
    Vec::new()
}

fn actions(ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) -> Vec<Command> {
    ui.overlay = Some(Overlay::Menu(worktree_menu(ui, app, hits, metrics)));
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
    act.into_iter().collect()
}

/// What another surface answers for: the board's own rows, or the rail's.
fn elsewhere(
    target: Option<&Target>,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Option<Vec<Command>> {
    let target = target?;
    board::acted(target, ui, app)
        .or_else(|| rail::acted(target, ui))
        .or_else(|| agent::acted(target, point, ui, app, hits, metrics))
        .or_else(|| shell::acted(target, ui, app, metrics))
        .or_else(|| settings::acted(target, ui, app, hits))
        .or_else(|| resources::acted(target, ui, app, hits))
}
