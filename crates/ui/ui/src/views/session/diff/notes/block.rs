//! One shown note's block on the rows of its file.

use groove_controllers::AppState;
use groove_types::Note;

use super::{BETWEEN, Block, PADS, anchored, starts};
use crate::views::session::Face;
use crate::views::session::diff::wrap::wrapped;

/// One note's block, on the rows of the file it is about.
pub(super) fn block(
    app: &AppState,
    view: Face,
    (note, at, folded): (&Note, usize, bool),
    cols: usize,
) -> Option<Block> {
    let anchor = note.anchor.as_ref()?;
    let after = anchored(app, view, anchor)?;
    let said = if folded {
        Vec::new()
    } else {
        wrapped(note, cols)
    };
    let rows = match folded {
        true => 1 + PADS,
        false => said.len().max(1) + BETWEEN + 1 + PADS,
    };
    Some(Block {
        after,
        from: anchored(app, view, &starts(anchor)).unwrap_or(after),
        rows,
        at: Some(at),
        acts: !folded,
        typing: 0,
        folded,
        said,
    })
}
