//! What a note row says: the words, who said them, and the lines they are about.

use groove_controllers::AppState;

use super::notes::{Inline, Slot, lines};
use super::wrap::Worded;
use crate::Ui;
use crate::components::{Field, Header};
use crate::views::session::Noting;

#[derive(Default)]
pub(super) struct Words {
    pub(super) author: String,
    pub(super) lines: String,
    pub(super) body: String,
    pub(super) prose: Option<groove_ui_kit::markdown::Row>,
    pub(super) head: Option<Header>,
    pub(super) field: Option<Field>,
    /// The caret's column and what is held, on a row of the reply being typed.
    pub(super) caret: Option<usize>,
    pub(super) held: Option<(usize, usize, bool)>,
}

/// Who said what on a note row; a row of code says nothing.
pub(super) fn words_of(app: &AppState, ui: &Ui, slot: Slot, inline: &Inline) -> Words {
    match slot {
        Slot::Note { at, row } => match (app.delivery.shown.get(at), inline.said(at, row)) {
            (Some(note), Worded::Head(said)) => Words {
                head: Some(header(note, said)),
                ..Words::default()
            },
            (Some(_), Worded::Body(prose)) => Words {
                prose: Some(prose),
                ..Words::default()
            },
            (None, _) => Words::default(),
        },
        Slot::Typed { row } => match ui.session.noting.as_ref() {
            Some(noting) => typed(noting, (row, inline), ui.focus == crate::Focus::Workspace),
            None => Words::default(),
        },
        Slot::Footer | Slot::Acts { .. } | Slot::Pad { .. } | Slot::Code(_) => Words::default(),
    }
}

/// One row of the reply being typed: its piece of a line, the caret and what is held on it.
fn typed(noting: &Noting, (row, inline): (usize, &Inline), lit: bool) -> Words {
    let Some(piece) = inline.piece(row) else {
        return Words::default();
    };
    let (body, caret, held) = super::wrap::marked(&noting.buffer, piece);
    Words {
        field: Some(Field {
            first: row == 0,
            last: row + 1 == inline.typed_rows(),
            lit,
        }),
        held,
        caret,
        body,
        ..Words::default()
    }
}

/// The header of a note's comment `at`, its lines on the opening one.
fn header(note: &groove_types::Note, at: usize) -> Header {
    let lines = match at {
        0 => note.anchor.as_ref().map(lines).unwrap_or_default(),
        _ => String::new(),
    };
    Header::of(note, at, lines)
}
