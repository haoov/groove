//! The notes in the surface: the row each block follows, and the rows it takes.

use std::ops::Range;

use groove_controllers::AppState;
use groove_types::{Anchor, DiffView, Note};

use super::wrap::{cols_of, wrapped};
use crate::Ui;

/// One note's rows, standing after the row its last line sits on.
struct Block {
    after: usize,
    /// The first row the note is about.
    from: usize,
    rows: usize,
    /// Which note of the workspace it draws; none while it is being typed.
    at: Option<usize>,
    /// The note's own row of buttons, under what it says.
    acts: bool,
}

/// A row of the surface: a row of the view, one row of a note under it, the buttons
/// of a note, or the note being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Code(usize),
    Note { at: usize, row: usize },
    Acts { at: usize },
    Typed,
}

/// Where the notes stand among the rows of one view.
#[derive(Default)]
pub(crate) struct Inline {
    blocks: Vec<Block>,
}

impl Inline {
    /// The notes of this view, each on the row its file shows it on.
    pub(crate) fn of(app: &AppState, ui: &Ui, view: DiffView) -> Self {
        if app.workspace.commit.is_some() {
            return Self::default();
        }
        let over = ui.session.noting.as_ref().and_then(|one| one.over_id());
        let mut blocks: Vec<Block> = app
            .workspace
            .notes
            .iter()
            .enumerate()
            .filter(|(_, note)| !written(note, over))
            .filter_map(|(at, note)| block(app, view, note, at, cols_of(ui)))
            .collect();
        if let Some(noting) = ui.session.noting.as_ref()
            && let Some(after) = anchored(app, view, &noting.anchor)
        {
            blocks.push(Block {
                after,
                from: anchored(app, view, &starts(&noting.anchor)).unwrap_or(after),
                rows: 1,
                at: None,
                acts: false,
            });
        }
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
                return (block.slot(row - start), block.after);
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

    /// Whether a note of the surface stands on this row of the view.
    pub(crate) fn notes(&self, code: usize) -> bool {
        self.blocks
            .iter()
            .any(|block| block.at.is_some() && (block.from..=block.after).contains(&code))
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

impl Block {
    /// What one row of the block holds, counted from its first.
    fn slot(&self, row: usize) -> Slot {
        let Some(at) = self.at else {
            return Slot::Typed;
        };
        match self.acts && row + 1 == self.rows {
            true => Slot::Acts { at },
            false => Slot::Note { at, row },
        }
    }
}

/// Whether this note is the one being written again.
fn written(note: &Note, over: Option<&groove_types::AnnotationId>) -> bool {
    over.is_some_and(|id| note.id() == Some(id))
}

/// One note's block, on the rows of the file it is about.
fn block(app: &AppState, view: DiffView, note: &Note, at: usize, cols: usize) -> Option<Block> {
    let anchor = note.anchor.as_ref()?;
    let after = anchored(app, view, anchor)?;
    Some(Block {
        after,
        from: anchored(app, view, &starts(anchor)).unwrap_or(after),
        rows: wrapped(note, cols).len().max(1) + 1,
        at: Some(at),
        acts: true,
    })
}

/// The anchor's own first line, as an anchor of its own.
fn starts(anchor: &Anchor) -> Anchor {
    Anchor::line(anchor.path.clone(), anchor.start_line)
}

/// The row an anchor's last line stands on, in the view that draws it.
fn anchored(app: &AppState, view: DiffView, anchor: &Anchor) -> Option<usize> {
    match view {
        DiffView::Editor => {
            let open = app.workspace.opened.as_ref()?;
            (open.path == anchor.path && (anchor.end_line as usize) < open.new.lines().max(1))
                .then_some(anchor.end_line as usize)
        }
        _ => app.workspace.changes.row_of(&anchor.path, anchor.end_line),
    }
}

/// What one row of a note says: who said it, and the words on that row.
pub(crate) fn said(note: &Note, row: usize, cols: usize) -> (String, String) {
    wrapped(note, cols).into_iter().nth(row).unwrap_or_default()
}

/// The lines an anchor covers, as a file numbers them.
pub(crate) fn lines(anchor: &Anchor) -> String {
    let (from, to) = (anchor.start_line + 1, anchor.end_line + 1);
    match from == to {
        true => from.to_string(),
        false => format!("{from}-{to}"),
    }
}
