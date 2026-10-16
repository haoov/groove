//! One shown note's block on the rows of its file.

use groove_controllers::AppState;
use groove_types::Note;

use super::{BETWEEN, Block, anchored, starts};
use crate::views::session::Face;
use crate::views::session::diff::wrap::wrapped;

/// One note's block, on the rows of the file it is about; a hidden one takes none.
pub(super) fn block(
    app: &AppState,
    (view, lines): (Face, Option<&[usize]>),
    (note, at, hidden): (&Note, usize, bool),
    cols: usize,
) -> Option<Block> {
    let anchor = note.anchor.as_ref()?;
    let after = anchored(app, (view, lines), anchor)?;
    let said = if hidden {
        Vec::new()
    } else {
        wrapped(note, cols)
    };
    let rows = match hidden {
        true => 0,
        false => said.len().max(1) + BETWEEN + 1,
    };
    Some(Block {
        after,
        from: anchored(app, (view, lines), &starts(anchor)).unwrap_or(after),
        rows,
        at: Some(at),
        acts: !hidden,
        typing: 0,
        said,
    })
}

impl super::Inline {
    /// The piece of the note being typed that row `row` of its field shows.
    pub(crate) fn piece(&self, row: usize) -> Option<(usize, usize, usize)> {
        self.typed.get(row).copied()
    }

    /// How many rows the note being typed takes.
    pub(crate) fn typed_rows(&self) -> usize {
        self.typed.len()
    }
}
