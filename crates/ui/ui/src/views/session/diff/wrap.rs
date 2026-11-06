//! A note's words laid out on rows as wide as the surface gives them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use groove_controllers::workspace_service::Buffer;
use groove_types::{Note, labelled};
use groove_ui_kit::markdown::{Row, compact};

use groove_ui_kit::base::tokens::{NOTE_COLS, NOTE_COLS_FEWEST};

/// How many characters a note row holds, from what a frame measured; none before the first.
pub(crate) fn cols_of(measured: usize) -> usize {
    match measured {
        0 => NOTE_COLS,
        cols => cols.max(NOTE_COLS_FEWEST),
    }
}

/// One row of a note: the header a comment opens on, or a line of what it says.
#[derive(Clone, Debug)]
pub(crate) enum Worded {
    /// The header of the note's comment at this index.
    Head(usize),
    Body(Row),
}

impl Default for Worded {
    fn default() -> Self {
        Self::Body(Row::default())
    }
}

/// How many laid-out comments the cache keeps before it starts again.
const KEPT: usize = 4096;

/// A comment's words laid out.
type Laid = Rc<Vec<Row>>;

/// Each comment's words laid out at a width, kept across frames.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Compacted {
    kept: RefCell<HashMap<(String, usize), Laid>>,
}

impl Compacted {
    fn of(&self, body: &str, cols: usize) -> Laid {
        let mut kept = self.kept.borrow_mut();
        if let Some(rows) = kept.get(&(body.to_string(), cols)) {
            return rows.clone();
        }
        if kept.len() >= KEPT {
            kept.clear();
        }
        let rows = Rc::new(compact(body, cols));
        kept.insert((body.to_string(), cols), rows.clone());
        rows
    }
}

/// Every row a note takes: each comment's header, then its words as Markdown, its label off.
pub(crate) fn wrapped(note: &Note, cols: usize, compacted: &Compacted) -> Vec<Worded> {
    let mut out = Vec::new();
    for (at, said) in note.said.iter().enumerate() {
        out.push(Worded::Head(at));
        let body = labelled(&said.body).map_or(said.body.as_str(), |one| one.rest);
        let rows = compacted.of(body, cols);
        out.extend(rows.iter().cloned().map(Worded::Body));
    }
    out
}

/// The rows typed text takes at `cols`: each one's line, and its first and past-last character,
/// a long line broken after its last space that fits.
pub(crate) fn pieces(text: &str, cols: usize) -> Vec<(usize, usize, usize)> {
    let cols = cols.max(1);
    let mut out = Vec::new();
    for (line, body) in text.split('\n').enumerate() {
        let chars: Vec<char> = body.chars().collect();
        let mut start = 0;
        loop {
            let mut end = (start + cols).min(chars.len());
            if end < chars.len()
                && let Some(space) = chars[start..end].iter().rposition(|c| *c == ' ')
                && space > 0
            {
                end = start + space + 1;
            }
            out.push((line, start, end));
            if end >= chars.len() {
                break;
            }
            start = end;
        }
    }
    out
}

/// A row's words, the caret's column on it, and what is held on it.
pub(crate) type Marked = (String, Option<usize>, Option<(usize, usize, bool)>);

pub(crate) fn marked(buffer: &Buffer, (line, start, end): (usize, usize, usize)) -> Marked {
    let whole = buffer.line(line).unwrap_or_default();
    let chars = whole.chars().count();
    let tail = end >= chars;
    let body: String = whole.chars().skip(start).take(end - start).collect();
    let caret = buffer.caret();
    let on = caret.line == line && caret.column >= start && (caret.column < end || tail);
    let held = buffer
        .selections()
        .first()
        .and_then(|one| one.on(line, chars));
    let held = held.and_then(|(from, to, through)| {
        let (from, to) = (from.max(start), to.min(end));
        (from < to || (from == to && to == end && through && tail)).then_some((
            from - start,
            to - start,
            through && tail,
        ))
    });
    (body, on.then(|| caret.column - start), held)
}
