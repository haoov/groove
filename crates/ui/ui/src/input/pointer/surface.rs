//! Where a click in the rows lands: the caret, the file it names, the lens, a match.

mod composer;
mod noted;
mod tabs;

pub(super) use composer::composing;
pub(super) use noted::noted;

use groove_controllers::workspace_service::column_of;
use groove_controllers::{AppState, Command, workspace};
use groove_types::{Caret, DiffView, Edit, Motion, Selection};

use crate::components::{code_at, first};
use crate::editor::Editing;
use crate::hit::{Hits, Scroller, Target};
use crate::input::follow::reaching;
use crate::views::session::{Face, Tab, diff};
use crate::{Click, Focus, Ui};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::ABOVE_MATCH;
pub(crate) use tabs::middle;
pub(super) use tabs::{closing, in_files};

/// Changes the view, keeping the line at the top of the old one in view.
pub(super) fn switch(
    ui: &mut Ui,
    app: &AppState,
    view: DiffView,
    (hits, metrics): (&Hits, Metrics),
) {
    let line = metrics.tokens().code_line;
    let faces = (Face::Stream(ui.session.view), Face::Stream(view));
    ui.session.diff = diff::scrolled(app, ui, faces, (ui.session.diff, line), hits.wrap());
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
    let editing = Editing::keyed(app, ui).unwrap_or(Editing::File);
    if !editing.lands(app) {
        return Vec::new();
    }
    if !holds(app, ui, &path) {
        let open = workspace::Command::OpenFile {
            path,
            at: Some(Selection::at(caret)),
        };
        return vec![Command::Workspace(open)];
    }
    let mut edits = vec![Edit::Move(Motion::To(caret))];
    edits.extend(taken(ui.clicked));
    edits.into_iter().map(|edit| editing.edit(edit)).collect()
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
    ui.session.diff = head as f32 * metrics.tokens().code_line;
    true
}

/// The lens dragged to the pointer, holding the rows around where it points.
pub(super) fn lensed(y: f32, ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) {
    let Some(rect) = hits.rect_of(&Target::Map) else {
        return;
    };
    let total = diff::rows_of(app, ui, hits.wrap());
    if total == 0 {
        return;
    }
    let line = metrics.tokens().code_line;
    let at = ((y - rect.y) / rect.h).clamp(0.0, 1.0) * total as f32 * line;
    let far = hits.extent(Scroller::Code);
    *ui.session.scroll_mut() = (at - rect.h / 2.0).clamp(0.0, far);
}

/// The file of one found line, opened in the file view with that line held.
pub(super) fn reached(
    ui: &mut Ui,
    app: &AppState,
    (hits, metrics): (&Hits, Metrics),
    at: usize,
) -> Vec<Command> {
    let Some(one) = app.workspace.found.get(at) else {
        return Vec::new();
    };
    ui.focus = Focus::Workspace;
    ui.session.tab = Tab::Files;
    let above = one.line.saturating_sub(ABOVE_MATCH);
    ui.session.file = above as f32 * metrics.tokens().code_line;
    ui.session.file_across = reaching(&one.text, &one.path, one.at.1, hits, metrics);
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

/// The commit a blame names, shown as the change it made.
pub(super) fn blamed(ui: &mut Ui, sha: String) -> Vec<Command> {
    ui.session.tab = Tab::Diff;
    vec![Command::Workspace(workspace::Command::OpenCommit { sha })]
}

/// Whether the buffer being edited is this file.
pub(super) fn holds(app: &AppState, ui: &Ui, path: &str) -> bool {
    diff::edited(app, ui).is_some_and(|one| one.path == path)
}

/// A file's rows hidden or shown again, with what stands above them held still.
pub(super) fn folded(ui: &mut Ui, app: &AppState, metrics: Metrics, path: String) -> Vec<Command> {
    let changes = &app.workspace.changes;
    let line = metrics.tokens().code_line;
    let top = first(line, ui.session.diff);
    if let (Some(head), Some(file)) = (changes.head_of(&path), changes.get(&path))
        && head < top
    {
        let rows = file.len();
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
pub(in crate::input) fn at(
    ui: &Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Option<(String, Caret)> {
    let (row, display) = row_at(hits, metrics, point)?;
    let (path, line, from) = diff::line_at(app, ui, (ui.session.face(), hits.wrap()), row)?;
    let (text, width) = diff::text_at((app, ui), &path, line)?;
    Some((
        path,
        Caret::new(line, column_of(&text, from + display, width)),
    ))
}

/// The pointer moved while it holds text of the open file: the selection carried to it.
pub(super) fn extended(
    (x, y): (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let Some((path, caret)) = at(ui, app, hits, metrics, (x, y)) else {
        return Vec::new();
    };
    let editing = crate::editor::Editing::keyed(app, ui);
    match (holds(app, ui, &path), editing) {
        (true, Some(editing)) => vec![editing.edit(Edit::Extend(Motion::To(caret)))],
        _ => Vec::new(),
    }
}

/// A ctrl+click on the open file's code: where the symbol under the pointer is defined.
pub(super) fn defined(
    ui: &Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Vec<Command> {
    let Some((path, at)) = at(ui, app, hits, metrics, point) else {
        return Vec::new();
    };
    if app.workspace.buffer(&path).is_none() {
        return Vec::new();
    }
    vec![Command::Workspace(workspace::Command::Definition {
        path,
        at,
    })]
}

/// The block a line of the open file opens, folded under it or shown again.
pub(super) fn fold(ui: &mut Ui, app: &AppState, line: usize) -> Vec<Command> {
    if let Some(open) = crate::views::session::diff::edited(app, ui) {
        let folds = ui.session.folds.entry(open.path.to_string()).or_default();
        folds.seen = open.buffer.revision();
        if !folds.starts.remove(&line) {
            folds.starts.insert(line);
        }
    }
    Vec::new()
}
