//! A note's words laid out on rows as wide as the surface gives them.

use groove_types::Note;
use groove_ui_kit::markdown::{Row, compact};

use crate::Ui;

/// How many characters a note row holds before the frame has said.
const COLS: usize = 80;

/// How many characters a reply stands in by.
const REPLY_STEP: usize = 2;

/// The fewest columns a row is given.
const FEWEST: usize = 20;

/// How many characters a note row holds, as the last frame measured it.
pub(crate) fn cols_of(ui: &Ui) -> usize {
    match ui.session.note_cols {
        0 => COLS,
        cols => cols.max(FEWEST),
    }
}

/// Every row a note takes: its words as Markdown, each reply a blank row under and one step in.
pub(crate) fn wrapped(note: &Note, cols: usize) -> Vec<(String, Row)> {
    let mut out = Vec::new();
    for (reply, said) in note.said.iter().enumerate() {
        let rows = match reply {
            0 => compact(&said.body, cols),
            _ => {
                out.push((String::new(), Row::default()));
                compact(&said.body, cols.saturating_sub(REPLY_STEP))
                    .into_iter()
                    .map(|row| Row {
                        depth: row.depth + 1,
                        ..row
                    })
                    .collect()
            }
        };
        for (at, row) in rows.into_iter().enumerate() {
            let author = match at {
                0 => said.author.clone(),
                _ => String::new(),
            };
            out.push((author, row));
        }
    }
    out
}
