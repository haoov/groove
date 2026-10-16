//! The notes in the surface: the row each block follows, and the rows it takes.

use std::ops::Range;

use groove_controllers::AppState;
use groove_types::{Anchor, Note};

mod answer;
mod block;
mod tiles;

use self::answer::answering;
use self::block::block;
use super::wrap::{Worded, cols_of};
use crate::Ui;
use crate::components::Room;

pub(crate) use self::tiles::Tiled;
use crate::views::session::Face;

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
    /// The rows of a reply being typed, in place of the buttons.
    typing: usize,
    /// What each row of the note says, wrapped once for the frame.
    said: Vec<Worded>,
}

/// A row of the surface: a view row, a note row, a note's buttons, or the note being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Code(usize),
    Note {
        at: usize,
        row: usize,
    },
    Acts {
        at: usize,
    },
    /// A row of the words being typed; `reply` when they answer the note they stand in.
    Typed {
        row: usize,
        reply: bool,
    },
    /// The labels and decorations a note being written picks from.
    Chips,
    /// What the note being typed offers: save and cancel.
    Footer {
        reply: bool,
    },
    /// A half row of room around a note's card, or across it.
    Pad {
        room: Room,
    },
}

/// The rows of room the composer keeps around its label chips, one above and one below.
const GAPS: usize = 2;
/// The row that rules off what a note says from what it offers.
const BETWEEN: usize = 1;

/// Where the notes stand among the rows of one view.
#[derive(Default)]
pub(crate) struct Inline {
    blocks: Vec<Block>,
    /// The note being typed, as its rows show it.
    typed: Vec<(usize, usize, usize)>,
}

impl Inline {
    /// The notes of this view, each on the row its file shows it on, wrapped to `cols`.
    pub(crate) fn of(app: &AppState, ui: &Ui, view: Face, cols: usize) -> Self {
        let unfiled =
            view == Face::File && super::row::edited(app, ui).is_some_and(|one| one.file.is_none());
        if app.workspace.commit.is_some() || unfiled {
            return Self::default();
        }
        let wrapped = super::wrapping::wrap_of(app, ui).map(|(starts, _)| starts);
        let lines = wrapped.as_deref().map(Vec::as_slice);
        let noting = ui.session.noting.as_ref();
        let typed = noting.map_or_else(Vec::new, |one| {
            super::wrap::pieces(&one.buffer.text(), cols_of(cols))
        });
        let over = noting.and_then(|one| one.over_id());
        let mut blocks: Vec<Block> = app
            .delivery
            .shown
            .iter()
            .enumerate()
            .filter(|(_, note)| !written(note, over))
            .filter_map(|(at, note)| {
                let hidden = note.resolved && !ui.session.unfolded.contains(&note.origin);
                block(app, (view, lines), (note, at, hidden), cols_of(cols))
            })
            .collect();
        let answered = noting.and_then(|one| answering(app, one, (&mut blocks, typed.len())));
        if let Some(noting) = noting.filter(|_| answered.is_none())
            && let Some(after) = anchored(app, (view, lines), &noting.anchor)
        {
            let typing = typed.len();
            blocks.push(Block {
                after,
                from: anchored(app, (view, lines), &starts(&noting.anchor)).unwrap_or(after),
                rows: GAPS + 1 + typing + BETWEEN + 1,
                at: None,
                acts: false,
                typing,
                said: Vec::new(),
            });
        }
        blocks.sort_by_key(|block| (block.after, block.at));
        Self { blocks, typed }
    }

    /// What one row of a note holds: a comment's header, or a line of its words.
    pub(crate) fn said(&self, at: usize, row: usize) -> Worded {
        self.blocks
            .iter()
            .find(|block| block.at == Some(at))
            .and_then(|block| block.said.get(row).cloned())
            .unwrap_or_default()
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

    /// Whether a row opens its note's card, and whether it closes it.
    pub(crate) fn edges(&self, row: usize) -> (bool, bool) {
        let mut shift = 0;
        for block in &self.blocks {
            let start = block.after + 1 + shift;
            if row >= start && row < start + block.rows {
                return (row == start, row + 1 == start + block.rows);
            }
            shift += block.rows;
        }
        (false, false)
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
            .filter(|block| block.at.is_some() && block.rows > 0)
            .any(|block| (block.from..=block.after).contains(&code))
    }

    /// Which of `slots` a note stands on.
    pub(crate) fn noted(&self, slots: &[Slot]) -> Vec<bool> {
        slots
            .iter()
            .map(|slot| matches!(slot, Slot::Code(at) if self.notes(*at)))
            .collect()
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
            let typed = GAPS + 1;
            return match row {
                0 | 2 => Slot::Pad { room: Room::Gap },
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

/// Whether this note is the one being written again.
fn written(note: &Note, over: Option<&groove_types::AnnotationId>) -> bool {
    over.is_some_and(|id| note.id() == Some(id))
}

/// The anchor's own first line, as an anchor of its own.
fn starts(anchor: &Anchor) -> Anchor {
    Anchor::line(anchor.path.clone(), anchor.start_line)
}

/// Whether the note being typed is drawn: no commit shown, and its line in the view.
pub(crate) fn noting_drawn(app: &AppState, ui: &Ui) -> bool {
    let Some(noting) = ui.session.noting.as_ref() else {
        return false;
    };
    let wrapped = super::wrapping::wrap_of(app, ui).map(|(starts, _)| starts);
    app.workspace.commit.is_none()
        && anchored(
            app,
            (ui.session.face(), wrapped.as_deref().map(Vec::as_slice)),
            &noting.anchor,
        )
        .is_some()
}

/// The row an anchor's last line stands on, in the view that draws it.
fn anchored(
    app: &AppState,
    (view, wrapped): (Face, Option<&[usize]>),
    anchor: &Anchor,
) -> Option<usize> {
    match view {
        Face::File => {
            let open = app.workspace.active()?;
            let line = anchor.end_line as usize;
            let shown = open.path == anchor.path && line < open.new.lines().max(1);
            let last = |starts: &[usize]| starts.get(line + 1).map_or(line, |next| next - 1);
            shown.then(|| wrapped.map_or(line, last))
        }
        _ => app.workspace.changes.row_of(&anchor.path, anchor.end_line),
    }
}

/// The lines an anchor covers, as a file numbers them.
pub(crate) fn lines(anchor: &Anchor) -> String {
    let (from, to) = (anchor.start_line + 1, anchor.end_line + 1);
    match from == to {
        true => from.to_string(),
        false => format!("{from}-{to}"),
    }
}
