//! The file view's lines wrapped to its width.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use groove_controllers::AppState;
use groove_controllers::workspace_service::display_of;

use super::row::{Drawn, edited, text_of};
use crate::Ui;
use crate::views::session::Face;

/// The first row of each line, then the row past the last.
pub(crate) type Starts = Rc<Vec<usize>>;

/// What the starts were built for: the buffer's revision, its lines, the width, the file.
type Key = (u64, usize, usize, String);

/// The lines' starts last built, what they were built for, and the width the last frame gave.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Wrapping {
    kept: RefCell<Option<(Key, Starts)>>,
    cols: Cell<usize>,
}

impl Wrapping {
    /// The characters a row of code holds, as the frame measured it.
    pub(crate) fn measured(&self, cols: usize) {
        self.cols.set(cols);
    }

    pub(crate) fn cols(&self) -> usize {
        self.cols.get()
    }
}

/// The file view's wrap, while wrapping is on and the view has been measured.
pub(crate) fn wrap_of(app: &AppState, ui: &Ui) -> Option<(Starts, usize)> {
    let cols = ui.wrapping.cols();
    if !app.config.wrap_code() || ui.session.face() != Face::File || cols == 0 {
        return None;
    }
    let editor = edited(app, ui)?;
    let buffer = editor.buffer;
    let key = (
        buffer.revision(),
        buffer.lines(),
        cols,
        editor.path.to_string(),
    );
    let mut kept = ui.wrapping.kept.borrow_mut();
    if let Some((at, starts)) = kept.as_ref()
        && *at == key
    {
        return Some((starts.clone(), cols));
    }
    let width = buffer.document().indent().width();
    let mut starts = Vec::with_capacity(buffer.lines() + 1);
    let mut row = 0;
    for line in 0..buffer.lines().max(1) {
        starts.push(row);
        let text = text_of(buffer, line);
        let wide = display_of(&text, text.chars().count(), width);
        row += wide.div_ceil(cols).max(1);
    }
    starts.push(row);
    let starts = Rc::new(starts);
    *kept = Some((key, starts.clone()));
    Some((starts, cols))
}

/// The line a row shows, and which of the line's rows it is.
pub(crate) fn line_of(starts: &[usize], row: usize) -> (usize, usize) {
    let line = starts
        .partition_point(|start| *start <= row)
        .saturating_sub(1);
    (line, row - starts[line])
}

/// One of a line's rows: its columns from `part * cols`, the numbers on its first row only.
pub(super) fn piece(whole: &Drawn, part: usize, cols: usize) -> Drawn {
    let (from, to) = (part * cols, (part + 1) * cols);
    let chars = whole.text.chars().count();
    let tail = to >= chars;
    let byte = |col: usize| {
        whole
            .text
            .char_indices()
            .nth(col)
            .map_or(whole.text.len(), |(at, _)| at)
    };
    let (start, end) = (byte(from.min(chars)), byte(to.min(chars)));
    let spans = whole
        .spans
        .iter()
        .filter(|one| one.range.end > start && one.range.start < end)
        .map(|one| groove_types::Highlight {
            range: one.range.start.max(start) - start..one.range.end.min(end) - start,
            capture: one.capture,
        })
        .collect();
    let clip = |(a, b): (usize, usize)| {
        let (a, b) = (a.max(from), b.min(to));
        (a < b).then_some((a - from, b - from))
    };
    Drawn {
        text: whole.text[start..end].to_string(),
        spans,
        gutters: match part {
            0 => whole.gutters.clone(),
            _ => vec![String::new(); whole.gutters.len()],
        },
        kind: whole.kind,
        caret: whole
            .caret
            .filter(|at| *at >= from && (*at < to || tail))
            .map(|at| at - from),
        held: whole.held.and_then(|(a, b, through)| {
            let (a, b) = (a.max(from), b.min(to));
            (a < b || (a == b && tail && through)).then_some((a - from, b - from, through && tail))
        }),
        mark: whole.mark,
        found: whole.found.iter().copied().filter_map(clip).collect(),
        words: whole.words.iter().copied().filter_map(clip).collect(),
        standing: whole.standing.and_then(clip),
        ..Drawn::default()
    }
}

/// The row the caret stands on, among the wrapped rows of the file view.
pub(crate) fn row_of(starts: &[usize], cols: usize, (line, display): (usize, usize)) -> usize {
    let (first, next) = (
        starts.get(line).copied().unwrap_or(0),
        starts.get(line + 1).copied(),
    );
    let parts = next.map_or(1, |next| next - first);
    first + (display / cols.max(1)).min(parts.saturating_sub(1))
}

/// The line and display column one wrapped row up or down.
pub(crate) fn stepped(
    starts: &[usize],
    cols: usize,
    (line, display): (usize, usize),
    down: bool,
) -> Option<(usize, usize)> {
    let row = row_of(starts, cols, (line, display));
    let next = match down {
        true => row + 1,
        false => row.checked_sub(1)?,
    };
    if next >= starts.last().copied().unwrap_or(0) {
        return None;
    }
    let (line, part) = line_of(starts, next);
    Some((line, part * cols + display % cols.max(1)))
}
