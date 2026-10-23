//! One shown note's block on the rows of its file.

use groove_controllers::AppState;
use groove_types::Note;

use super::{BETWEEN, Block, GAPS, OVER, Slot, UNDER, anchored, starts};
use crate::components::Room;
use crate::views::session::Face;
use crate::views::session::diff::wrap::{Compacted, wrapped};

/// One note's block, on the rows of the file it is about; a hidden one takes none.
pub(super) fn block(
    app: &AppState,
    (view, lines): (Face, Option<&[usize]>),
    (note, at, hidden): (&Note, usize, bool),
    (cols, compacted): (usize, &Compacted),
) -> Option<Block> {
    let anchor = note.anchor.as_ref()?;
    let after = anchored(app, (view, lines), anchor)?;
    let said = if hidden {
        Vec::new()
    } else {
        wrapped(note, cols, compacted)
    };
    let rows = match hidden {
        true => 0,
        false => OVER + said.len().max(1) + BETWEEN + 1 + UNDER,
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

impl Block {
    /// What one row of the block holds, counted from its first.
    pub(super) fn slot(&self, row: usize) -> Slot {
        if row + UNDER >= self.rows {
            return Slot::Pad { room: Room::Under };
        }
        let Some(at) = self.at else {
            let typed = GAPS + 1;
            return match row {
                0 => Slot::Pad { room: Room::Over },
                2 => Slot::Pad { room: Room::Gap },
                1 => Slot::Chips,
                row if row < typed + self.typing => Slot::Typed {
                    row: row - typed,
                    reply: false,
                },
                row if row == typed + self.typing => Slot::Pad {
                    room: Room::Between,
                },
                _ => Slot::Footer { reply: false },
            };
        };
        let Some(row) = row.checked_sub(OVER) else {
            return Slot::Pad { room: Room::Over };
        };
        let said = self.said.len().max(1);
        match row.checked_sub(said + BETWEEN) {
            None if row < said => Slot::Note { at, row },
            None => Slot::Pad {
                room: Room::Between,
            },
            Some(row) if row < self.typing => Slot::Typed { row, reply: true },
            Some(_) if self.typing > 0 => Slot::Footer { reply: true },
            Some(_) => Slot::Acts { at },
        }
    }
}

/// Each row of a note told it floats over the rows, on its card's glass.
pub(crate) fn floated(lines: &mut [crate::components::Line<'_>], slots: &[Slot]) {
    for (line, slot) in lines.iter_mut().zip(slots) {
        line.float = floats(slot);
    }
}

/// Whether a row belongs to a note, whose card floats over the rows on glass.
pub(crate) fn floats(slot: &Slot) -> bool {
    !matches!(slot, Slot::Code(_))
}
