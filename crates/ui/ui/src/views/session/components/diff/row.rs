//! What one row says: its text, its colours, its numbers and its mark.

use std::collections::BTreeMap;
use std::ops::Range;

use groove_controllers::AppState;
use groove_controllers::workspace_service::{Aligned, At, Colours, Opened};
use groove_types::{Caret, DiffView, Highlight, LineMark, Row, RowKind};

use crate::{Focus, Ui};
use groove_controllers::workspace_service::{display_at, shown};

/// Which file a row is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Old,
    New,
}

/// A row as the surface needs it, owned so the lines can borrow it.
#[derive(Default)]
pub(super) struct Drawn {
    pub(super) text: String,
    pub(super) spans: Vec<Highlight>,
    pub(super) gutters: Vec<String>,
    pub(super) kind: RowKind,
    /// The caret's column, when the caret is on this row.
    pub(super) caret: Option<usize>,
    /// The columns a selection covers on this row, and whether it carries on.
    pub(super) held: Option<(usize, usize, bool)>,
    /// What the file view says happened to this line.
    pub(super) mark: Option<LineMark>,
    /// The row a file starts on, which names it instead of showing a line.
    pub(super) head: bool,
}

/// The rows of one view, inside `window`. The file view is the open file; the others
/// are every changed file, one after another.
pub(super) fn drawn(
    app: &AppState,
    ui: &Ui,
    view: DiffView,
    side: Side,
    window: Range<usize>,
) -> Vec<Drawn> {
    match view {
        DiffView::File => whole(app, ui, window),
        _ => streamed(app, ui, view, side, window),
    }
}

/// How many rows the view stands, all of it.
pub(super) fn count(app: &AppState, view: DiffView) -> usize {
    match view {
        DiffView::File => open(app).map_or(0, |file| file.new.lines()),
        _ => app.workspace.changes.rows(),
    }
}

fn open(app: &AppState) -> Option<&Opened> {
    app.workspace.opened.as_ref()
}

/// The lines of the open file as it is now, marked where the change touched them.
fn whole(app: &AppState, ui: &Ui, window: Range<usize>) -> Vec<Drawn> {
    let Some(file) = open(app) else {
        return Vec::new();
    };
    let caret = caret(ui, file);
    let colours = file.new.colours(window.clone());
    let width = file.new.document().indent().width();
    window
        .map(|at| {
            let text = text_of(file, at);
            let (drawn, spans) = shown(&text, colours.of(at), width);
            Drawn {
                text: drawn,
                spans,
                gutters: vec![(at + 1).to_string()],
                kind: RowKind::Context,
                caret: caret
                    .filter(|on| on.line == at)
                    .map(|on| display_at(&text, on.column, width)),
                held: held(file, ui, at),
                mark: file.marks.get(&(at as u32)).copied(),
                head: false,
            }
        })
        .collect()
}

/// Every changed file's rows, each under a row naming the file.
fn streamed(
    app: &AppState,
    ui: &Ui,
    view: DiffView,
    side: Side,
    window: Range<usize>,
) -> Vec<Drawn> {
    let changes = &app.workspace.changes;
    let colours = coloured(app, window.clone());
    window
        .map(|row| match changes.at(row) {
            Some(At::Head(file)) => head(file),
            Some(At::Row(file, at)) => {
                let side = source(&file.rows[at], view, side);
                one(
                    app,
                    ui,
                    file,
                    at,
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
fn coloured(app: &AppState, window: Range<usize>) -> BTreeMap<&str, (Colours, Colours)> {
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
            Some((path, (before.colours(old), after.colours(new))))
        })
        .collect()
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
fn head(file: &Aligned) -> Drawn {
    Drawn {
        text: file.path.clone(),
        head: true,
        ..Drawn::default()
    }
}

/// One row of one file, read from the side the view asks for.
fn one(
    app: &AppState,
    ui: &Ui,
    file: &Aligned,
    at: usize,
    view: DiffView,
    side: Side,
    colours: Option<&(Colours, Colours)>,
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
        text: drawn,
        spans,
        gutters: gutters(row, view, side),
        kind: kind(row, view, side),
        caret: here.and_then(|open| on_row(caret(ui, open), row, open, file.indent)),
        held: here.and_then(|open| row.new.and_then(|line| held(open, ui, line as usize))),
        mark: None,
        head: false,
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

fn spans_of(colours: Option<&(Colours, Colours)>, side: Side, line: Option<u32>) -> Vec<Highlight> {
    let (Some(colours), Some(line)) = (colours, line) else {
        return Vec::new();
    };
    let found = match side {
        Side::Old => colours.0.of(line as usize),
        Side::New => colours.1.of(line as usize),
    };
    found.to_vec()
}

/// Where the caret is, while the workspace holds the keyboard.
fn caret(ui: &Ui, file: &Opened) -> Option<Caret> {
    match ui.focus == Focus::Workspace {
        true => Some(file.new.caret()),
        false => None,
    }
}

/// The file and line a row of the whole surface shows, on the new side.
pub(crate) fn line_at(app: &AppState, view: DiffView, row: usize) -> Option<(String, usize)> {
    match view {
        DiffView::File => {
            let file = open(app)?;
            (row < file.new.lines()).then(|| (file.path.clone(), row))
        }
        _ => match app.workspace.changes.at(row)? {
            At::Head(_) => None,
            At::Row(file, at) => {
                let line = file.rows[at].new?;
                Some((file.path.clone(), line as usize))
            }
        },
    }
}

/// A line of any changed file, and how wide a tab reads in it.
pub(crate) fn text_at(app: &AppState, path: &str, line: usize) -> Option<(String, usize)> {
    if let Some(file) = open(app).filter(|file| file.path == path) {
        let width = file.new.document().indent().width();
        return Some((text_of(file, line), width));
    }
    let file = app.workspace.changes.get(path)?;
    let at = file
        .rows
        .iter()
        .position(|row| row.new == Some(line as u32))?;
    Some((file.lines[at].clone(), file.indent))
}

/// What a caret holds on `line`, in the columns the row draws.
fn held(file: &Opened, ui: &Ui, line: usize) -> Option<(usize, usize, bool)> {
    if ui.focus != Focus::Workspace {
        return None;
    }
    let text = text_of(file, line);
    let chars = text.chars().count();
    let (from, to, through) = file
        .new
        .selections()
        .iter()
        .find_map(|one| one.on(line, chars))?;
    let width = file.new.document().indent().width();
    Some((
        display_at(&text, from, width),
        display_at(&text, to, width),
        through,
    ))
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
        (DiffView::File, _) => vec![number(row.new)],
        (DiffView::Inline, RowKind::Removed) => vec![number(row.old), String::new()],
        (DiffView::Inline, _) => vec![String::new(), number(row.new)],
    }
}

/// The line as it is, for counting columns over what is drawn.
fn text_of(file: &Opened, line: usize) -> String {
    file.new.line(line).unwrap_or_default().to_string()
}
