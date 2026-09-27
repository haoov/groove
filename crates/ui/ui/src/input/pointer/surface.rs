//! Where a click in the rows lands: the caret, the file it names, the lens, a match.

mod noted;

pub(super) use noted::noted;

use groove_controllers::workspace_service::columns;
use groove_controllers::{AppState, Command, workspace};
use groove_types::{Caret, DiffView, Edit, Motion, Selection};

use crate::components::{code_at, first};
use crate::hit::{Chars, Hits, Scroller, Target};
use crate::views::session::{Face, Tab, diff};
use crate::{Click, Focus, Ui};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::ABOVE_MATCH;

/// Changes the view, keeping the line at the top of the old one in view.
pub(super) fn switch(ui: &mut Ui, app: &AppState, view: DiffView, metrics: Metrics) {
    let line = metrics.tokens().line;
    let (from, to) = (Face::Stream(ui.session.view), Face::Stream(view));
    ui.session.diff = diff::scrolled(app, ui, from, to, ui.session.diff, line);
    ui.session.view = view;
}

/// Where a click in the open file puts the caret; a row with no new-side line takes none.
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
    if app.workspace.readonly() {
        return Vec::new();
    }
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

/// A changed file's rows shown again when they were folded away.
pub(super) fn unfolded(app: &AppState, path: String) -> Vec<Command> {
    match app.workspace.changes.is_folded(&path) {
        true => vec![Command::Workspace(workspace::Command::Fold { path })],
        false => Vec::new(),
    }
}

/// The stream scrolled to where this file starts, when the change holds it.
pub(super) fn jump(ui: &mut Ui, app: &AppState, path: &str, metrics: Metrics) -> bool {
    let Some(head) = app.workspace.changes.head_of(path) else {
        return false;
    };
    ui.session.diff = head as f32 * metrics.tokens().line;
    true
}

/// The file active in the Files tab, from its top when it was not already.
pub(super) fn in_files(ui: &mut Ui, app: &AppState, path: String) -> Vec<Command> {
    ui.session.tab = Tab::Files;
    if app.workspace.active().is_none_or(|one| one.path != path) {
        ui.session.file = 0.0;
    }
    vec![Command::Workspace(workspace::Command::OpenFile {
        path,
        at: None,
    })]
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
    *ui.session.scroll_mut() = (at - rect.h / 2.0).clamp(0.0, far);
}

/// The file of one found line, opened in the file view with that line held.
pub(super) fn reached(ui: &mut Ui, app: &AppState, metrics: Metrics, at: usize) -> Vec<Command> {
    let Some(one) = app.workspace.found.get(at) else {
        return Vec::new();
    };
    ui.focus = Focus::Workspace;
    ui.session.tab = Tab::Files;
    let above = one.line.saturating_sub(ABOVE_MATCH);
    ui.session.file = above as f32 * metrics.tokens().line;
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
    app.workspace.active().is_some_and(|open| open.path == path)
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
    let (path, line) = diff::line_at(app, ui, ui.session.face(), row)?;
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
        advance: metrics.advance,
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

/// A tab closed, or the question first when its file owes the disk.
pub(super) fn closing(ui: &mut Ui, app: &AppState, path: String) -> Vec<Command> {
    let owes = app
        .workspace
        .buffer(&path)
        .is_some_and(|one| one.new.dirty());
    if owes {
        ui.overlay = Some(crate::Overlay::Losing(crate::Losing::Tab(path)));
        return Vec::new();
    }
    vec![Command::Workspace(workspace::Command::CloseFile { path })]
}
