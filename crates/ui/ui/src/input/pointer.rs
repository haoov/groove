//! What a press, a drag and a click do, through what the last frame drew.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::{Caret, DiffView, Edit, Motion, Selection, WorktreeId};

use super::Key;
use crate::ctx::Metrics;
use crate::hit::{Hits, Picks, Scroller, Target};
use crate::layout::{Edge, Layout};
use crate::palette::{Action, Anchor, Flow, Palette};
use crate::tokens::{CLICK_MS, CLICK_SLOP};
use crate::views::session::components::diff;
use crate::widget::{code_at, first};
use crate::{Click, Corner, Drag, Focus, Losing, Menu, Of, Ui};
use groove_controllers::workspace_service::columns;

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

/// The right button on a file's row opens its actions; anywhere else closes them.
pub(super) fn asked(x: f32, y: f32, ui: &mut Ui, hits: &Hits, metrics: Metrics) {
    ui.discarding = None;
    ui.menu = match hits.at(x, y) {
        Some(Target::File(path)) => Some(Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of: Of::File(path),
        }),
        Some(Target::Actions) => Some(worktree_menu(ui, hits, metrics)),
        _ => None,
    };
}

/// This press, against the one before it: a press soon after another and near it
/// carries the same click on.
fn counted(last: Option<Click>, x: f32, y: f32, metrics: Metrics) -> Click {
    let slop = CLICK_SLOP * metrics.scale;
    let same = last.filter(|last| {
        metrics.tick.saturating_sub(last.at) <= CLICK_MS
            && (last.x - x).abs() <= slop
            && (last.y - y).abs() <= slop
    });
    Click {
        x,
        y,
        at: metrics.tick,
        count: same.map_or(1, |last| last.count + 1),
    }
}

/// Takes hold of `edge`, keeping how far from it the pointer landed.
fn grab(ui: &mut Ui, edge: Edge, x: f32, y: f32, metrics: Metrics) {
    let at = ui.split.edge_at(edge, window_of(metrics), sidebar(ui));
    ui.drag = Some(Drag {
        edge,
        offset: along(edge, x, y, metrics) - at,
    });
}

/// The pointer's place along the axis the boundary moves in, in logical pixels.
fn along(edge: Edge, x: f32, y: f32, metrics: Metrics) -> f32 {
    match edge.upright() {
        true => logical(x, metrics),
        false => logical(y, metrics),
    }
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

/// The boundary follows the pointer.
fn drag_to(ui: &mut Ui, x: f32, y: f32, metrics: Metrics) {
    let Some(drag) = ui.drag else {
        return;
    };
    let at = along(drag.edge, x, y, metrics) - drag.offset;
    ui.split
        .drag(drag.edge, at, window_of(metrics), sidebar(ui));
}

fn sidebar(ui: &Ui) -> bool {
    ui.session.sidebar()
}

/// The window's width in logical pixels.
fn window_of(metrics: Metrics) -> (f32, f32) {
    let rect = metrics.size.rect();
    (logical(rect.w, metrics), logical(rect.h, metrics))
}

fn logical(value: f32, metrics: Metrics) -> f32 {
    value / metrics.scale
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
    match target {
        Some(Target::Session(session)) => {
            vec![Command::Session(session::Command::Select { session })]
        }
        Some(Target::Tab(tab)) => {
            ui.session.tab = tab;
            Vec::new()
        }
        Some(Target::Fold) => {
            ui.session.folded = !ui.session.folded;
            Vec::new()
        }
        Some(Target::Picker(which)) => selector(ui, app, hits, which),
        Some(Target::Worktree(worktree)) => select_worktree(app, worktree),
        Some(Target::File(path)) => {
            jump(ui, app, &path, metrics);
            shown(app, path)
        }
        Some(Target::View(view)) => {
            switch(ui, app, view, metrics);
            Vec::new()
        }
        Some(Target::Code) => {
            ui.selecting = true;
            landed(ui, app, hits, metrics, (x, y))
        }
        Some(Target::Read(path)) => {
            vec![Command::Workspace(workspace::Command::MarkRead { path })]
        }
        Some(Target::Head(path)) => folded(ui, app, metrics, path),
        Some(Target::Pinned) => Vec::new(),
        Some(Target::Map) => {
            ui.mapping = true;
            lensed(y, ui, app, hits, metrics);
            Vec::new()
        }

        Some(Target::Stage(path)) => {
            vec![Command::Workspace(workspace::Command::Stage { path })]
        }
        Some(Target::Unstage(path)) => {
            vec![Command::Workspace(workspace::Command::Unstage { path })]
        }
        Some(Target::Discard) => lose(ui),
        Some(Target::Keep) => {
            ui.discarding = None;
            Vec::new()
        }
        Some(Target::Actions) => {
            ui.menu = Some(worktree_menu(ui, hits, metrics));
            Vec::new()
        }
        Some(Target::Message) => {
            ui.session.composing = true;
            Vec::new()
        }
        Some(Target::Do) => {
            ui.session.composing = false;
            let act = crate::views::session::components::commit::primary(app);
            act.map(Command::Workspace).into_iter().collect()
        }
        Some(Target::PaletteRow(at)) => palette_row(at, ui, app),
        Some(Target::Agent | Target::Palette | Target::Split(_) | Target::MenuRow(_)) | None => {
            Vec::new()
        }
    }
}

/// The worktree's actions stand above the caret that opened them, ending on the rule
/// that separates the box from the list.
fn worktree_menu(ui: &Ui, hits: &Hits, metrics: Metrics) -> Menu {
    let box_ = Layout::of(metrics, ui).commit;
    let right = hits
        .rect_of(&Target::Actions)
        .map(|caret| caret.right())
        .unwrap_or(box_.right());
    Menu {
        at: (right, box_.y),
        corner: Corner::BottomRight,
        of: Of::Worktree,
    }
}

/// The answer that throws the change away: one file's, or every one.
fn lose(ui: &mut Ui) -> Vec<Command> {
    let asked = ui.discarding.take();
    let command = match asked {
        Some(Losing::File(path)) => workspace::Command::Discard { path },
        Some(Losing::Everything) => workspace::Command::DiscardAll,
        None => return Vec::new(),
    };
    vec![Command::Workspace(command)]
}

/// A click while a menu is open: a row of it, or anywhere to close it.
fn chosen(target: Option<Target>, ui: &mut Ui) -> Vec<Command> {
    let menu = ui.menu.take();
    let (Some(Target::MenuRow(at)), Some(menu)) = (target, menu) else {
        return Vec::new();
    };
    let (commands, asking) = crate::views::shared::actions::picked(&menu.of, at);
    ui.discarding = asking;
    commands
}

/// Changes the view, keeping the line at the top of the old one in view.
fn switch(ui: &mut Ui, app: &AppState, view: DiffView, metrics: Metrics) {
    let line = metrics.tokens().line;
    let from = ui.session.view;
    ui.session.diff = diff::scrolled(app, from, view, ui.session.diff, line);
    ui.session.view = view;
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

/// Where a click in the open file puts the caret. A row the new side has no line
/// on — a removed one, a gap — takes no caret.
fn landed(
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Vec<Command> {
    let Some((path, caret)) = at(ui, app, hits, metrics, point) else {
        return Vec::new();
    };
    if !holds(app, &path) {
        let open = workspace::Command::OpenFile {
            path,
            at: Some(Selection::at(caret)),
        };
        return vec![Command::Workspace(open)];
    }
    let mut edits = vec![Edit::Move(Motion::To(caret))];
    edits.extend(taken(ui.clicked));
    edits
        .into_iter()
        .map(|edit| Command::Workspace(workspace::Command::Edit(edit)))
        .collect()
}

/// What a click of its own takes: one lands the caret, two a word, three the line.
fn taken(click: Option<Click>) -> Option<Edit> {
    match click?.count {
        2 => Some(Edit::SelectWord),
        count if count >= 3 => Some(Edit::SelectLine),
        _ => None,
    }
}

/// The file opened, and its rows shown again when it was folded away.
fn shown(app: &AppState, path: String) -> Vec<Command> {
    let mut commands = Vec::new();
    if app.workspace.changes.is_folded(&path) {
        let fold = workspace::Command::Fold { path: path.clone() };
        commands.push(Command::Workspace(fold));
    }
    let open = workspace::Command::OpenFile { path, at: None };
    commands.push(Command::Workspace(open));
    commands
}

/// The stream scrolled to where this file starts.
fn jump(ui: &mut Ui, app: &AppState, path: &str, metrics: Metrics) {
    if ui.session.view == DiffView::File {
        return;
    }
    let Some(head) = app.workspace.changes.head_of(path) else {
        return;
    };
    ui.session.diff = head as f32 * metrics.tokens().line;
}

/// The lens dragged to the pointer, holding the rows around where it points.
fn lensed(y: f32, ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) {
    let Some(rect) = hits.rect_of(&Target::Map) else {
        return;
    };
    let total = diff::rows_of(app, ui);
    if total == 0 {
        return;
    }
    let line = metrics.tokens().line;
    let at = ((y - rect.y) / rect.h).clamp(0.0, 1.0) * total as f32 * line;
    let far = hits.extent(Scroller::Code);
    ui.session.diff = (at - rect.h / 2.0).clamp(0.0, far);
}

/// Whether the buffer being edited is this file.
fn holds(app: &AppState, path: &str) -> bool {
    app.workspace
        .opened
        .as_ref()
        .is_some_and(|open| open.path == path)
}

/// A click on the lines standing above the rows: the file's own line folds it.
/// A file's rows hidden or shown again, with what stands above them held still.
fn folded(ui: &mut Ui, app: &AppState, metrics: Metrics, path: String) -> Vec<Command> {
    let changes = &app.workspace.changes;
    let line = metrics.tokens().line;
    let top = first(line, ui.session.diff);
    if let (Some(head), Some(file)) = (changes.head_of(&path), changes.get(&path))
        && head < top
    {
        let rows = file.rows.len();
        ui.session.diff = match changes.is_folded(&path) {
            true => ui.session.diff + rows as f32 * line,
            false => top.saturating_sub(rows).max(head) as f32 * line,
        };
    }
    vec![Command::Workspace(workspace::Command::Fold { path })]
}

/// The row of the whole surface a point lands on, and the column in it.
fn row_at(hits: &Hits, metrics: Metrics, point: (f32, f32)) -> Option<(usize, usize)> {
    let rect = hits.rect_of(&Target::Code)?;
    code_at(&metrics.tokens(), hits.chars(), rect, point)
}

/// The file a click in the rows points at, and where the caret lands in it.
fn at(
    ui: &Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Option<(String, Caret)> {
    let (row, display) = row_at(hits, metrics, point)?;
    let (path, line) = diff::line_at(app, ui.session.view, row)?;
    let (text, width) = diff::text_at(app, &path, line)?;
    Some((path, Caret::new(line, columns(&text, display, width))))
}

/// Either picker opens the worktree selector, under the picker itself.
fn selector(ui: &mut Ui, app: &AppState, hits: &Hits, which: Picks) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let flow = Flow::new(Action::SelectWorktree, session);
    let commands = flow.refresh(app).into_iter().collect();
    ui.palette = Some(Palette {
        flow: Some(flow),
        anchor: hits.rect_of(&Target::Picker(which)).map(Anchor::under),
        ..Palette::default()
    });
    commands
}

fn select_worktree(app: &AppState, worktree: WorktreeId) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::SelectWorktree {
        session,
        worktree,
    })]
}

/// A click on a row is that row selected, then confirmed.
fn palette_row(at: usize, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(palette) = &mut ui.palette else {
        return Vec::new();
    };
    palette.selected = at;
    let outcome = palette.key(Key::Enter, app);
    if outcome.close {
        ui.palette = None;
    }
    outcome.commands
}
