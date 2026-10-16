//! A note's words laid out on rows as wide as the surface gives them.

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

/// Every row a note takes: each comment's header, then its words as Markdown, its label off.
pub(crate) fn wrapped(note: &Note, cols: usize) -> Vec<Worded> {
    let mut out = Vec::new();
    for (at, said) in note.said.iter().enumerate() {
        out.push(Worded::Head(at));
        let body = labelled(&said.body).map_or(said.body.as_str(), |one| one.rest);
        out.extend(compact(body, cols).into_iter().map(Worded::Body));
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
