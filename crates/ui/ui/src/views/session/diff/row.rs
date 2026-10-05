//! What one row says: its text, its colours, its numbers and its mark.

mod stream;

use std::ops::Range;

use groove_controllers::AppState;
use groove_controllers::workspace_service::{At, Opened, display_of, shown};
use groove_types::{Caret, Highlight, LineMark, RowKind};

use self::stream::streamed;
use crate::views::session::Face;
use crate::{Focus, Ui};

/// Which file a row is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    Old,
    New,
}

/// A row as the surface needs it, owned.
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

/// The rows of one view inside `window`: the open file, or every changed file in turn.
pub(super) fn drawn(
    app: &AppState,
    ui: &Ui,
    view: Face,
    side: Side,
    window: Range<usize>,
) -> Vec<Drawn> {
    match view {
        Face::File => whole(app, ui, window),
        Face::Stream(view) => streamed(app, ui, view, side, window),
    }
}

/// How many rows the view stands, all of it.
pub(super) fn count(app: &AppState, view: Face) -> usize {
    match view {
        Face::File => open(app).map_or(0, |file| file.new.lines().max(1)),
        _ => app.workspace.changes.rows(),
    }
}

pub(super) fn open(app: &AppState) -> Option<&Opened> {
    app.workspace.active()
}

/// The lines of the open file as it is now, marked where the change touched them.
fn whole(app: &AppState, ui: &Ui, window: Range<usize>) -> Vec<Drawn> {
    let Some(file) = open(app) else {
        return Vec::new();
    };
    let caret = caret(ui, file);
    let stamp = (app.workspace.stamp, file.new.painted());
    let doc = file.new.document();
    let colours = ui.painted.of(doc, &file.path, false, stamp, window.clone());
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
                    .map(|on| display_of(&text, on.column, width)),
                held: held(file, ui, at),
                mark: file.hunked.marks.get(&(at as u32)).copied(),
                words: columns_in(file.hunked.words.new.get(&(at as u32)), &text, width),
                found: matched(ui, at, &text, width),
                standing: standing(ui, at, &text, width),
                ..Drawn::default()
            }
        })
        .collect()
}

/// What a live search found on this row, in the columns it draws.
pub(super) fn matched(ui: &Ui, row: usize, text: &str, width: usize) -> Vec<(usize, usize)> {
    let Some(find) = ui.session.find.as_ref().filter(|_| !text.is_empty()) else {
        return Vec::new();
    };
    find.on(row)
        .into_iter()
        .map(|at| columns_of(at, text, width))
        .collect()
}

/// The match the search stands on, when it stands on this row.
pub(super) fn standing(ui: &Ui, row: usize, text: &str, width: usize) -> Option<(usize, usize)> {
    let find = ui.session.find.as_ref().filter(|_| !text.is_empty())?;
    let at = find.standing(row)?;
    Some(columns_of(at, text, width))
}

/// Character ranges as the columns the row draws; a blank side draws none.
pub(super) fn columns_in(
    ranges: Option<&Vec<Range<usize>>>,
    text: &str,
    width: usize,
) -> Vec<(usize, usize)> {
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
        display_of(text, at.start, width),
        display_of(text, at.end, width),
    )
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

/// Where the caret is, while the workspace holds the keyboard.
pub(super) fn caret(ui: &Ui, file: &Opened) -> Option<Caret> {
    let here = ui.focus == Focus::Workspace && !ui.session.typing();
    here.then(|| file.new.caret())
}

/// The file and new-side line a row shows; a note's row shows none.
pub(crate) fn line_at(
    app: &AppState,
    ui: &Ui,
    (view, cols): (Face, usize),
    row: usize,
) -> Option<(String, usize)> {
    let super::notes::Slot::Code(row) = super::notes::Inline::of(app, ui, view, cols).slot(row)
    else {
        return None;
    };
    match view {
        Face::File => {
            let file = open(app)?;
            (row < file.new.lines().max(1)).then(|| (file.path.clone(), row))
        }
        _ => match app.workspace.changes.at(row)? {
            At::Head(_) => None,
            At::Row(file, at) => {
                let line = file.row(at)?.new?;
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
    let at = file.hunked.layout.position(line as u32)?;
    Some((file.text(at), file.indent))
}

/// What a caret holds on `line`, in the columns the row draws.
pub(super) fn held(file: &Opened, ui: &Ui, line: usize) -> Option<(usize, usize, bool)> {
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
        display_of(&text, from, width),
        display_of(&text, to, width),
        through,
    ))
}

/// The line as it is, for counting columns over what is drawn.
pub(super) fn text_of(file: &Opened, line: usize) -> String {
    file.new.line(line).unwrap_or_default().to_string()
}
