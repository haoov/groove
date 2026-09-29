//! A note's words laid out on rows as wide as the surface gives them.

use groove_types::Note;
use groove_ui_kit::markdown::{Row, compact};

use crate::Ui;

/// How many characters a note row holds before the frame has said.
const COLS: usize = 80;

/// The fewest columns a row is given.
const FEWEST: usize = 20;

/// How many characters a note row holds, as the last frame measured it.
pub(crate) fn cols_of(ui: &Ui) -> usize {
    match ui.session.note_cols {
        0 => COLS,
        cols => cols.max(FEWEST),
    }
}

/// Every row a note takes: who said it on each reply's first row, then its words as Markdown.
pub(crate) fn wrapped(note: &Note, cols: usize) -> Vec<(String, Row)> {
    let mut out = Vec::new();
    for said in &note.said {
        for (at, row) in compact(&said.body, cols).into_iter().enumerate() {
            let author = match at {
                0 => said.author.clone(),
                _ => String::new(),
            };
            out.push((author, row));
        }
    }
    out
}
