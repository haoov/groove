//! The file view's lines wrapped to its width, and a folded block's lines taking no row.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use groove_controllers::AppState;
use groove_controllers::workspace_service::display_of;

use super::row::{Drawn, edited, text_of};
use crate::Ui;
use crate::views::session::Face;

/// The first row of each line, then the row past the last.
pub(crate) type Starts = Rc<Vec<usize>>;

/// What the starts were built for: the buffer's revision, its lines, the width, the file, the folds.
type Key = (u64, usize, usize, String, Rc<Vec<super::blocks::Block>>);

/// The width a line is taken to have while lines do not wrap: wide enough for any.
pub(crate) const UNWRAPPED: usize = 1 << 30;

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

/// The file view's rows, while its lines wrap or a block is folded; `UNWRAPPED` when they do not wrap.
pub(crate) fn wrap_of(app: &AppState, ui: &Ui) -> Option<(Starts, usize)> {
    let measured = ui.wrapping.cols();
    let wraps = app.config.wrap_code() && measured > 0;
    let folded = super::blocks::folded(app, ui);
    if ui.session.face() != Face::File || (!wraps && folded.is_empty()) {
        return None;
    }
    let cols = if wraps { measured } else { UNWRAPPED };
    let editor = edited(app, ui)?;
    let buffer = editor.buffer;
    let key = (
        buffer.revision(),
        buffer.lines(),
        cols,
        editor.path.to_string(),
        folded.clone(),
    );
    let mut kept = ui.wrapping.kept.borrow_mut();
    if let Some((at, starts)) = kept.as_ref()
        && *at == key
    {
        return Some((starts.clone(), cols));
    }
    let width = buffer.document().indent().width();
    let lines = buffer.lines().max(1);
    let hidden = super::blocks::hidden(&folded, lines);
    let mut starts = Vec::with_capacity(lines + 1);
    let mut row = 0;
    for (line, hidden) in hidden.into_iter().enumerate() {
        starts.push(row);
        row += match (hidden, wraps) {
            (true, _) => 0,
            (false, false) => 1,
            (false, true) => {
                let text = text_of(buffer, line);
                display_of(&text, text.chars().count(), width)
                    .div_ceil(cols)
                    .max(1)
            }
        };
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
    let spans = groove_types::spans_within(&whole.spans, start..end);
    let clip = |(a, b): (usize, usize)| {
        let (a, b) = (a.max(from), b.min(to));
        (a < b).then(|| (a - from, b - from))
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
            (a < b || (a == b && tail && through)).then(|| (a - from, b - from, through && tail))
        }),
        mark: whole.mark,
        found: whole.found.iter().copied().filter_map(clip).collect(),
        words: whole.words.iter().copied().filter_map(clip).collect(),
        standing: whole.standing.and_then(clip),
        faults: within(&whole.faults, (from, to)),
        outline: outlined(&whole.outline, part, clip),
        ..Drawn::default()
    }
}

/// A piece's share of its line's outline: the occurrences it covers, the guides and fold on its first.
fn outlined(
    whole: &super::row::Outline,
    part: usize,
    clip: impl Fn((usize, usize)) -> Option<(usize, usize)>,
) -> super::row::Outline {
    super::row::Outline {
        occurs: whole.occurs.iter().copied().filter_map(clip).collect(),
        guides: match part {
            0 => whole.guides.clone(),
            _ => Vec::new(),
        },
        fold: whole.fold.filter(|_| part == 0),
    }
}

/// The faults over columns `from` to `to`, counted from `from`.
fn within(
    faults: &[super::faults::Fault],
    (from, to): (usize, usize),
) -> Vec<super::faults::Fault> {
    faults
        .iter()
        .filter_map(|(a, b, severity)| {
            let (a, b) = ((*a).max(from), (*b).min(to));
            (a < b).then(|| (a - from, b - from, *severity))
        })
        .collect()
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
