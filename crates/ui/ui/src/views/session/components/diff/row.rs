//! What one row says: its text, its colours, its numbers and its mark.

use std::ops::Range;

use groove_controllers::workspace_service::Opened;
use groove_types::{DiffView, Highlight, LineMark, Row, RowKind};

use crate::{Focus, Ui};

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
    pub(super) caret: bool,
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
fn whole(file: &Opened, ui: &Ui, window: Range<usize>) -> Vec<Drawn> {
    let caret = caret(ui);
    window
        .map(|at| Drawn {
            text: file.new.line(at).unwrap_or_default().to_string(),
            spans: file.new.spans(at),
            gutters: vec![(at + 1).to_string()],
            kind: RowKind::Context,
            caret: caret == Some(at),
            mark: file.marks.get(&(at as u32)).copied(),
        })
        .collect()
}

/// The alignment's rows, read from the side the view asks for.
fn aligned(file: &Opened, ui: &Ui, view: DiffView, side: Side, window: Range<usize>) -> Vec<Drawn> {
    let caret = caret(ui);
    let first = window.start;
    file.rows[window]
        .iter()
        .enumerate()
        .map(|(at, row)| {
            let (text, spans) = line(file, row, source(row, view, side));
            Drawn {
                text,
                spans,
                gutters: gutters(row, view, side),
                kind: kind(row, view, side),
                caret: caret == Some(first + at),
                mark: None,
            }
        })
        .collect()
}

/// The row the caret is on, while the workspace holds the keyboard.
fn caret(ui: &Ui) -> Option<usize> {
    match ui.focus == Focus::Workspace {
        true => ui.session.at.map(|(row, _)| row),
        false => None,
    }
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

/// The row's line and its colours. The ground says whether it came or went.
fn line(file: &Opened, row: &Row, side: Side) -> (String, Vec<Highlight>) {
    if let RowKind::Gap(lines) = row.kind {
        return (format!("\u{2026} {lines} lines"), Vec::new());
    }
    let (document, at) = match side {
        Side::Old => (&file.old, row.old),
        Side::New => (&file.new, row.new),
    };
    let Some(at) = at.map(|at| at as usize) else {
        return (String::new(), Vec::new());
    };
    let text = document.line(at).unwrap_or_default();
    (text.to_string(), document.spans(at))
}
