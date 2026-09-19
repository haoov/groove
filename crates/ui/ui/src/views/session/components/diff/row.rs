//! What one row says: its text, its colours, its numbers and its mark.

use std::ops::Range;

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_types::{Caret, DiffView, Highlight, LineMark, Row, RowKind};

use crate::{Focus, Ui};
use groove_controllers::workspace_service::{Colours, display_at, shown};

/// Which file a row is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Old,
    New,
}

/// A row as the surface needs it, owned so the lines can borrow it.
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
}

/// The rows of one view, inside `window`. The file view is the file; the others
/// are the alignment.
pub(super) fn drawn(
    file: &Opened,
    ui: &Ui,
    view: DiffView,
    side: Side,
    window: Range<usize>,
) -> Vec<Drawn> {
    match view {
        DiffView::File => whole(file, ui, window),
        _ => aligned(file, ui, view, side, window),
    }
}

/// How many rows the view stands, all of it.
pub(super) fn count(file: &Opened, view: DiffView) -> usize {
    match view {
        DiffView::File => file.new.lines(),
        _ => file.rows.len(),
    }
}

/// The lines of the file as it is now, marked where the change touched them.
pub(super) fn whole(file: &Opened, ui: &Ui, window: Range<usize>) -> Vec<Drawn> {
    let caret = caret(ui, file);
    let colours = file.new.colours(window.clone());
    window
        .map(|at| {
            let (text, spans) = shown(&text_of(file, at), colours.of(at), width(file));
            Drawn {
                text,
                spans,
                gutters: vec![(at + 1).to_string()],
                kind: RowKind::Context,
                caret: caret
                    .filter(|on| on.line == at)
                    .map(|on| display_at(&text_of(file, at), on.column, width(file))),
                held: held(file, ui, at),
                mark: file.marks.get(&(at as u32)).copied(),
            }
        })
        .collect()
}

/// The alignment's rows, read from the side the view asks for.
pub(super) fn aligned(
    file: &Opened,
    ui: &Ui,
    view: DiffView,
    side: Side,
    window: Range<usize>,
) -> Vec<Drawn> {
    let caret = caret(ui, file).filter(|_| side == Side::New);
    let rows = &file.rows[window];
    let (old, new) = bounds(rows);
    let (old, new) = (file.old.colours(old), file.new.colours(new));
    rows.iter()
        .map(|row| {
            let (text, spans) = line(file, row, source(row, view, side), (&old, &new));
            Drawn {
                text,
                spans,
                gutters: gutters(row, view, side),
                kind: kind(row, view, side),
                caret: on_row(caret, row).map(|column| {
                    let line = row.new.unwrap_or_default() as usize;
                    display_at(&text_of(file, line), column, width(file))
                }),
                held: row.new.and_then(|line| held(file, ui, line as usize)),
                mark: None,
            }
        })
        .collect()
}

/// Where the caret is, while the workspace holds the keyboard.
fn caret(ui: &Ui, file: &Opened) -> Option<Caret> {
    match ui.focus == Focus::Workspace {
        true => Some(file.new.caret()),
        false => None,
    }
}

/// The new-side line a row shows, if it shows one at all.
pub(crate) fn line_at(app: &AppState, view: DiffView, row: usize) -> Option<usize> {
    let file = app.workspace.opened.as_ref()?;
    match view {
        DiffView::File => (row < file.new.lines()).then_some(row),
        _ => file.rows.get(row)?.new.map(|line| line as usize),
    }
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
    let width = width(file);
    Some((
        display_at(&text, from, width),
        display_at(&text, to, width),
        through,
    ))
}

/// The caret's column when this row is the line it sits on. A removed line belongs
/// to the old document and takes no caret.
fn on_row(caret: Option<Caret>, row: &Row) -> Option<usize> {
    let caret = caret?;
    let shows = row.new == Some(caret.line as u32) && row.kind != RowKind::Removed;
    shows.then_some(caret.column)
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

/// The lines each side shows in `rows`, as one range a side.
fn bounds(rows: &[Row]) -> (Range<usize>, Range<usize>) {
    let ends = |pick: fn(&Row) -> Option<u32>| {
        let lines = rows.iter().filter_map(pick).map(|at| at as usize);
        lines.fold(None, |range: Option<Range<usize>>, at| match range {
            Some(range) => Some(range.start.min(at)..range.end.max(at + 1)),
            None => Some(at..at + 1),
        })
    };
    let range = |found: Option<Range<usize>>| found.unwrap_or(0..0);
    (range(ends(|row| row.old)), range(ends(|row| row.new)))
}

/// The row's line and its colours. The ground says whether it came or went.
fn line(
    file: &Opened,
    row: &Row,
    side: Side,
    colours: (&Colours, &Colours),
) -> (String, Vec<Highlight>) {
    if let RowKind::Gap(lines) = row.kind {
        return (format!("\u{2026} {lines} lines"), Vec::new());
    }
    let (document, at, colours) = match side {
        Side::Old => (&file.old, row.old, colours.0),
        Side::New => (file.new.document(), row.new, colours.1),
    };
    let Some(at) = at.map(|at| at as usize) else {
        return (String::new(), Vec::new());
    };
    let text = document.line(at).unwrap_or_default();
    shown(&text, colours.of(at), width(file))
}

/// How wide a tab reads in this file.
fn width(file: &Opened) -> usize {
    file.new.document().indent().width()
}

/// The line as it is, for counting columns over what is drawn.
fn text_of(file: &Opened, line: usize) -> String {
    file.new.line(line).unwrap_or_default().to_string()
}
