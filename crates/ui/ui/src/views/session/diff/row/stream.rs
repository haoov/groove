//! The rows of the whole change: every file under its own head, in the view's own side.

use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Aligned, At, Colours, Opened, display_at, shown};
use groove_types::{Caret, DiffView, Highlight, Row, RowKind};

use super::{Drawn, Side, caret, columns_in, held, is_read, matched, open, standing, text_of};
use crate::Ui;

/// Every changed file's rows, each under a row naming the file.
pub(super) fn streamed(
    app: &AppState,
    ui: &Ui,
    view: DiffView,
    side: Side,
    window: Range<usize>,
) -> Vec<Drawn> {
    let changes = &app.workspace.changes;
    let colours = coloured(app, ui, window.clone());
    window
        .map(|row| match changes.at(row) {
            Some(At::Head(file)) => head(app, file, changes.is_folded(&file.path)),
            Some(At::Row(file, at)) => {
                let side = source(&file.rows[at], view, side);
                one(
                    app,
                    ui,
                    file,
                    (row, at),
                    view,
                    side,
                    colours.get(file.path.as_str()),
                )
            }
            None => Drawn::default(),
        })
        .collect()
}

/// The colours of every file the window touches, asked for once a file a side.
fn coloured<'a>(
    app: &'a AppState,
    ui: &Ui,
    window: Range<usize>,
) -> BTreeMap<&'a str, (Rc<Colours>, Rc<Colours>)> {
    let changes = &app.workspace.changes;
    let mut bounds: BTreeMap<&str, (Range<usize>, Range<usize>)> = BTreeMap::new();
    for row in window {
        let Some(At::Row(file, at)) = changes.at(row) else {
            continue;
        };
        let (old, new) = bounds.entry(file.path.as_str()).or_default();
        stretch(old, file.rows[at].old);
        stretch(new, file.rows[at].new);
    }
    bounds
        .into_iter()
        .filter_map(|(path, (old, new))| {
            let (before, after) = app.workspace.sides(path)?;
            let stamp = (app.workspace.stamp, revision(app, path));
            let before = ui.painted.of(before, path, true, (stamp.0, 0), old);
            let after = ui.painted.of(after, path, false, stamp, new);
            Some((path, (before, after)))
        })
        .collect()
}

/// How many times the buffer of this file has changed, when it is the open one.
fn revision(app: &AppState, path: &str) -> u64 {
    open(app)
        .filter(|open| open.path == path)
        .map_or(0, |open| open.new.revision())
}

/// The range grown to hold one more line.
fn stretch(range: &mut Range<usize>, line: Option<u32>) {
    let Some(at) = line.map(|at| at as usize) else {
        return;
    };
    *range = match range.start == range.end {
        true => at..at + 1,
        false => range.start.min(at)..range.end.max(at + 1),
    };
}

/// The row that names a file, standing above its own rows.
fn head(app: &AppState, file: &Aligned, folded: bool) -> Drawn {
    Drawn {
        text: file.path.clone(),
        head: true,
        folded,
        read: is_read(app, &file.path),
        file: Some(file.path.clone()),
        ..Drawn::default()
    }
}

/// One row of one file, read from the side the view asks for.
fn one(
    app: &AppState,
    ui: &Ui,
    file: &Aligned,
    (on, at): (usize, usize),
    view: DiffView,
    side: Side,
    colours: Option<&(Rc<Colours>, Rc<Colours>)>,
) -> Drawn {
    let row = &file.rows[at];
    let line = match side {
        Side::Old => row.old,
        Side::New => row.new,
    };
    let here = open(app).filter(|open| open.path == file.path);
    let text = match blank(row, view, side) {
        true => String::new(),
        false => live(here, side, line).unwrap_or_else(|| file.lines[at].clone()),
    };
    let spans = spans_of(colours, side, line);
    let (drawn, spans) = shown(&text, &spans, file.indent);
    Drawn {
        words: columns_in(file.words.get(&at), &text, file.indent),
        found: matched(ui, on, &text, file.indent),
        standing: standing(ui, on, &text, file.indent),
        text: drawn,
        spans,
        gutters: gutters(row, view, side),
        kind: kind(row, view, side),
        caret: here.and_then(|open| on_row(caret(ui, open), row, open, file.indent)),
        held: here.and_then(|open| row.new.and_then(|line| held(open, ui, line as usize))),
        ..Drawn::default()
    }
}

/// The row's line as the buffer holds it now, for the file being edited.
fn live(open: Option<&Opened>, side: Side, line: Option<u32>) -> Option<String> {
    let (open, line) = (open?, line? as usize);
    let text = match side {
        Side::Old => open.old.line(line),
        Side::New => open.new.line(line),
    };
    Some(text?.to_string())
}

/// A row with nothing on this side shows nothing, whatever its own side holds.
fn blank(row: &Row, view: DiffView, side: Side) -> bool {
    let has = match side {
        Side::Old => row.old.is_some(),
        Side::New => row.new.is_some(),
    };
    view == DiffView::Split && !has
}

fn spans_of(
    colours: Option<&(Rc<Colours>, Rc<Colours>)>,
    side: Side,
    line: Option<u32>,
) -> Vec<Highlight> {
    let (Some(colours), Some(line)) = (colours, line) else {
        return Vec::new();
    };
    let found = match side {
        Side::Old => colours.0.of(line as usize),
        Side::New => colours.1.of(line as usize),
    };
    found.to_vec()
}

/// The caret's column when this row is the line it sits on. A removed line belongs
/// to the old document and takes no caret.
fn on_row(caret: Option<Caret>, row: &Row, file: &Opened, width: usize) -> Option<usize> {
    let caret = caret?;
    let shows = row.new == Some(caret.line as u32) && row.kind != RowKind::Removed;
    let text = text_of(file, caret.line);
    shows.then(|| display_at(&text, caret.column, width))
}

/// Which file a row's line is read from: a pane's own side in split, and in the
/// other views whichever side the row belongs to.
fn source(row: &Row, view: DiffView, side: Side) -> Side {
    match (view, row.kind) {
        (DiffView::Split, _) => side,
        (_, RowKind::Removed) => Side::Old,
        _ => Side::New,
    }
}

/// In split a row with nothing on this side draws blank and carries no ground.
fn kind(row: &Row, view: DiffView, side: Side) -> RowKind {
    match (view, row.kind, side) {
        (DiffView::Split, RowKind::Added, Side::Old) => RowKind::Context,
        (DiffView::Split, RowKind::Removed, Side::New) => RowKind::Context,
        _ => row.kind,
    }
}

/// Where the line sits: one number a side in split, otherwise one row's own.
fn gutters(row: &Row, view: DiffView, side: Side) -> Vec<String> {
    let number = |at: Option<u32>| at.map(|at| (at + 1).to_string()).unwrap_or_default();
    match (view, row.kind) {
        (_, RowKind::Gap(_)) => Vec::new(),
        (DiffView::Split, _) => match side {
            Side::Old => vec![number(row.old)],
            Side::New => vec![number(row.new)],
        },
        (DiffView::Editor, _) => vec![number(row.new)],
        (DiffView::Inline, RowKind::Removed) => vec![number(row.old), String::new()],
        (DiffView::Inline, _) => vec![String::new(), number(row.new)],
    }
}
