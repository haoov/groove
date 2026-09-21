//! The notes in the surface: the row each block follows, and the rows it takes.

use std::ops::Range;

use groove_controllers::AppState;
use groove_types::{DiffView, Note};

/// One note's rows, standing after the row its last line sits on.
struct Block {
    after: usize,
    rows: usize,
    /// Which note of the workspace it draws.
    at: usize,
}

/// A row of the surface: a row of the view, or one row of a note under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Code(usize),
    Note { at: usize, row: usize },
}

/// Where the notes stand among the rows of one view.
#[derive(Default)]
pub(crate) struct Inline {
    blocks: Vec<Block>,
}

impl Inline {
    /// The notes of this view, each on the row its file shows it on.
    pub(crate) fn of(app: &AppState, view: DiffView) -> Self {
        let mut blocks: Vec<Block> = app
            .workspace
            .notes
            .iter()
            .enumerate()
            .filter_map(|(at, note)| {
                Some(Block {
                    after: row_of(app, view, note)?,
                    rows: note.said.len().max(1),
                    at,
                })
            })
            .collect();
        blocks.sort_by_key(|block| (block.after, block.at));
        Self { blocks }
    }

    /// How many rows the surface stands with the notes in it.
    pub(crate) fn total(&self, code: usize) -> usize {
        code + self.blocks.iter().map(|block| block.rows).sum::<usize>()
    }

    /// What one row of the surface holds.
    pub(crate) fn slot(&self, row: usize) -> Slot {
        self.found(row).0
    }

    /// The row of the view a surface row belongs to; a note row belongs to its line.
    pub(crate) fn base(&self, row: usize) -> usize {
        self.found(row).1
    }

    /// What the row holds, and the row of the view it belongs to.
    fn found(&self, row: usize) -> (Slot, usize) {
        let mut shift = 0;
        for block in &self.blocks {
            let start = block.after + 1 + shift;
            if row < start {
                break;
            }
            if row < start + block.rows {
                let slot = Slot::Note {
                    at: block.at,
                    row: row - start,
                };
                return (slot, block.after);
            }
            shift += block.rows;
        }
        (Slot::Code(row - shift), row - shift)
    }

    /// Where a row of the view stands once the notes take their room.
    pub(crate) fn shifted(&self, code: usize) -> usize {
        let taken: usize = self
            .blocks
            .iter()
            .filter(|block| block.after < code)
            .map(|block| block.rows)
            .sum();
        code + taken
    }

    /// The rows of the view that `window` covers, as the builders ask for them.
    pub(crate) fn code_window(&self, window: Range<usize>) -> Range<usize> {
        let mut bounds: Option<Range<usize>> = None;
        for row in window {
            let Slot::Code(at) = self.slot(row) else {
                continue;
            };
            bounds = Some(match bounds {
                Some(held) => held.start.min(at)..held.end.max(at + 1),
                None => at..at + 1,
            });
        }
        bounds.unwrap_or(0..0)
    }
}

/// The row a note's last line stands on, in the view that draws it.
fn row_of(app: &AppState, view: DiffView, note: &Note) -> Option<usize> {
    let anchor = note.anchor.as_ref()?;
    match view {
        DiffView::Editor => {
            let open = app.workspace.opened.as_ref()?;
            (open.path == anchor.path && (anchor.end_line as usize) < open.new.lines().max(1))
                .then_some(anchor.end_line as usize)
        }
        _ => app.workspace.changes.row_of(&anchor.path, anchor.end_line),
    }
}

/// What one row of a note says: who said it, and the words themselves.
pub(crate) fn said(note: &Note, row: usize) -> (String, String) {
    match note.said.get(row) {
        Some(said) => (said.author.clone(), one_line(&said.body)),
        None => (String::new(), String::new()),
    }
}

/// A body on one row: its first line, and a mark that more of it stands under.
fn one_line(body: &str) -> String {
    let mut lines = body.lines();
    let first = lines.next().unwrap_or_default().trim();
    match lines.next().is_some() {
        true => format!("{first} \u{2026}"),
        false => first.to_string(),
    }
}
