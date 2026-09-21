//! Where a click in the rows lands: the caret, the file it names, the lens, a match.

use groove_controllers::workspace_service::columns;
use groove_controllers::{AppState, Command, workspace};
use groove_types::{Caret, DiffView, Edit, Motion, Selection};

use crate::ctx::Metrics;
use crate::hit::{Chars, Hits, Scroller, Target};
use crate::tokens::ABOVE_MATCH;
use crate::views::session::diff;
use crate::widget::{code_at, first};
use crate::{Click, Focus, Ui};

/// Changes the view, keeping the line at the top of the old one in view.
pub(super) fn switch(ui: &mut Ui, app: &AppState, view: DiffView, metrics: Metrics) {
    let line = metrics.tokens().line;
    let from = ui.session.view;
    ui.session.diff = diff::scrolled(app, from, view, ui.session.diff, line);
    ui.session.view = view;
}

/// Where a click in the open file puts the caret. A row the new side has no line
/// on — a removed one, a gap — takes no caret.
pub(super) fn landed(
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
pub(super) fn taken(click: Option<Click>) -> Option<Edit> {
    match click?.count {
        2 => Some(Edit::SelectWord),
        count if count >= 3 => Some(Edit::SelectLine),
        _ => None,
    }
}

/// The file opened, and its rows shown again when it was folded away.
pub(super) fn shown(app: &AppState, path: String) -> Vec<Command> {
    let mut commands = Vec::new();
    if app.workspace.changes.is_folded(&path) {
        let fold = workspace::Command::Fold { path: path.clone() };
        commands.push(Command::Workspace(fold));
    }
    let open = workspace::Command::OpenFile { path, at: None };
    commands.push(Command::Workspace(open));
    commands
}

/// The stream scrolled to where this file starts; one the change lacks becomes a file.
pub(super) fn jump(ui: &mut Ui, app: &AppState, path: &str, metrics: Metrics) {
    if ui.session.view == DiffView::File {
        return;
    }
    let Some(head) = app.workspace.changes.head_of(path) else {
        ui.session.view = DiffView::File;
        return;
    };
    ui.session.diff = head as f32 * metrics.tokens().line;
}

/// The lens dragged to the pointer, holding the rows around where it points.
pub(super) fn lensed(y: f32, ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) {
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

/// The file one found line belongs to, opened and stood on with that line held. A
/// search reaches files the change never touched, which only the file view shows.
pub(super) fn reached(ui: &mut Ui, app: &AppState, metrics: Metrics, at: usize) -> Vec<Command> {
    let Some(one) = app.workspace.found.get(at) else {
        return Vec::new();
    };
    ui.focus = Focus::Workspace;
    ui.session.view = DiffView::File;
    let above = one.line.saturating_sub(ABOVE_MATCH);
    ui.session.diff = above as f32 * metrics.tokens().line;
    let held = Selection {
        anchor: Caret::new(one.line, one.at.0),
        head: Caret::new(one.line, one.at.1),
    };
    let open = workspace::Command::OpenFile {
        path: one.path.clone(),
        at: Some(held),
    };
    vec![Command::Workspace(open)]
}

/// Whether the buffer being edited is this file.
pub(super) fn holds(app: &AppState, path: &str) -> bool {
    app.workspace
        .opened
        .as_ref()
        .is_some_and(|open| open.path == path)
}

/// A file's rows hidden or shown again, with what stands above them held still.
pub(super) fn folded(ui: &mut Ui, app: &AppState, metrics: Metrics, path: String) -> Vec<Command> {
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
pub(super) fn row_at(hits: &Hits, metrics: Metrics, point: (f32, f32)) -> Option<(usize, usize)> {
    let rect = hits.rect_of(&Target::Code)?;
    code_at(&metrics.tokens(), hits.chars(), rect, point)
}

/// The file a click in the rows points at, and where the caret lands in it.
pub(super) fn at(
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

/// Where a click in the commit box puts its caret.
pub(super) fn composed(
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Vec<Command> {
    let Some(rect) = hits.rect_of(&Target::Message) else {
        return Vec::new();
    };
    let chars = Chars {
        left: rect.x,
        advance: metrics.cell.width,
        scroll: 0.0,
    };
    let Some((row, display)) = code_at(&metrics.tokens(), chars, rect, point) else {
        return Vec::new();
    };
    let message = &app.workspace.message;
    let line = row.min(message.lines().saturating_sub(1));
    let text = message.line(line).unwrap_or_default();
    let width = message.document().indent().width();
    let caret = Caret::new(line, columns(&text, display, width));
    vec![Command::Workspace(workspace::Command::Message(Edit::Move(
        Motion::To(caret),
    )))]
}
