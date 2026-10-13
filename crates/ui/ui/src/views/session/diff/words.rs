//! What a note row says: the words, who said them, and the lines they are about.

use groove_controllers::AppState;

use super::notes::{Inline, Slot, lines};
use crate::Ui;
use crate::views::session::Noting;
use groove_controllers::delivery_service::BY_USER;

#[derive(Default)]
pub(super) struct Words {
    pub(super) author: String,
    pub(super) lines: String,
    pub(super) body: String,
    pub(super) prose: Option<groove_ui_kit::markdown::Row>,
    /// The caret's column and what is held, on a row of the note being typed.
    pub(super) caret: Option<usize>,
    pub(super) held: Option<(usize, usize, bool)>,
}

/// Who said what on a note row; a row of code says nothing.
pub(super) fn words_of(app: &AppState, ui: &Ui, slot: Slot, inline: &Inline) -> Words {
    match slot {
        Slot::Note { at, row } => match app.delivery.shown.get(at) {
            Some(note) => {
                let (author, prose) = inline.said(at, row);
                let shown = match row {
                    0 => note.anchor.as_ref().map(lines).unwrap_or_default(),
                    _ => String::new(),
                };
                Words {
                    author,
                    lines: shown,
                    prose: Some(prose),
                    ..Words::default()
                }
            }
            None => Words::default(),
        },
        Slot::Typed { row } => match ui.session.noting.as_ref() {
            Some(noting) => typed(noting, row),
            None => Words::default(),
        },
        Slot::Acts { .. } | Slot::Code(_) => Words::default(),
    }
}

/// One row of the note being typed: its line of the buffer, the caret and what is held on it.
fn typed(noting: &Noting, row: usize) -> Words {
    let buffer = &noting.buffer;
    let body = buffer.line(row).unwrap_or_default().to_string();
    let caret = buffer.caret();
    let chars = body.chars().count();
    let (author, lines) = match row {
        0 => (BY_USER.to_string(), lines(&noting.anchor)),
        _ => (String::new(), String::new()),
    };
    Words {
        author,
        lines,
        held: buffer
            .selections()
            .first()
            .and_then(|one| one.on(row, chars)),
        caret: (caret.line == row).then_some(caret.column),
        body,
        prose: None,
    }
}
