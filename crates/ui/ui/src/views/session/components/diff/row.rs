//! What one row says: its text, its colours, its numbers and its mark.

use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

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
    /// The row a directory starts on.
    pub(super) band: bool,
    /// A head row whose file hides its rows.
    pub(super) folded: bool,
    /// A head row whose file has been read.
    pub(super) read: bool,
    /// The file a head row names.
    pub(super) file: Option<String>,
    /// What a search found on this row, in the columns the row draws.
    pub(super) found: Vec<(usize, usize)>,
    /// The columns the row it pairs with does not have.
    pub(super) words: Vec<(usize, usize)>,
    /// The one of them the search stands on.
    pub(super) standing: Option<(usize, usize)>,
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
                words: columns_in(file.words.get(&(at as u32)), &text, width),
                found: matched(ui, at, &text, width),
                standing: standing(ui, at, &text, width),
                ..Drawn::default()
            }
        })
        .collect()
}

/// What a live search found on this row, in the columns the row draws. A side that
/// draws the row blank marks nothing.
fn matched(ui: &Ui, row: usize, text: &str, width: usize) -> Vec<(usize, usize)> {
    let Some(find) = ui.session.find.as_ref().filter(|_| !text.is_empty()) else {
        return Vec::new();
    };
    find.on(row)
        .into_iter()
        .map(|at| columns_of(at, text, width))
        .collect()
}

/// The match the search stands on, when it stands on this row.
fn standing(ui: &Ui, row: usize, text: &str, width: usize) -> Option<(usize, usize)> {
    let find = ui.session.find.as_ref().filter(|_| !text.is_empty())?;
    let at = find.standing(row)?;
    Some(columns_of(at, text, width))
}

/// Character ranges as the columns the row draws; a blank side draws none.
fn columns_in(ranges: Option<&Vec<Range<usize>>>, text: &str, width: usize) -> Vec<(usize, usize)> {
    let Some(ranges) = ranges.filter(|_| !text.is_empty()) else {
        return Vec::new();
    };
    ranges
        .iter()
        .map(|at| columns_of(at.clone(), text, width))
        .collect()
}

fn columns_of(at: Range<usize>, text: &str, width: usize) -> (usize, usize) {
    (
        display_at(text, at.start, width),
        display_at(text, at.end, width),
    )
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
    let colours = coloured(app, ui, window.clone());
    window
        .map(|row| match changes.at(row) {
            Some(At::Band(file)) => band(file),
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
        text: file.name().to_string(),
        head: true,
        folded,
        read: is_read(app, &file.path),
        file: Some(file.path.clone()),
        ..Drawn::default()
    }
}

/// Whether the session has marked this file of the worktree read.
pub(crate) fn is_read(app: &AppState, path: &str) -> bool {
    let Some(open) = app.session.selected() else {
        return false;
    };
    let Some(worktree) = open.selected_worktree() else {
        return false;
    };
    open.is_read(&worktree.id, path)
}

/// The row that names the directory the files under it share.
fn band(file: &Aligned) -> Drawn {
    Drawn {
        text: file.dir().to_string(),
        band: true,
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

/// Where the caret is, while the workspace holds the keyboard.
fn caret(ui: &Ui, file: &Opened) -> Option<Caret> {
    let here = ui.focus == Focus::Workspace && !ui.session.typing();
    here.then(|| file.new.caret())
}

/// The file and line a row of the whole surface shows, on the new side.
pub(crate) fn line_at(app: &AppState, view: DiffView, row: usize) -> Option<(String, usize)> {
    match view {
        DiffView::File => {
            let file = open(app)?;
            (row < file.new.lines()).then(|| (file.path.clone(), row))
        }
        _ => match app.workspace.changes.at(row)? {
            At::Band(_) | At::Head(_) => None,
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
