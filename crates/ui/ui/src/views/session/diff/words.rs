//! What a note row says: the words, who said them, and the lines they are about.

use groove_controllers::AppState;

use super::notes::{Inline, Slot, lines};
use crate::Ui;
use groove_controllers::delivery_service::BY_USER;

#[derive(Default)]
pub(super) struct Words {
    pub(super) author: String,
    pub(super) lines: String,
    pub(super) body: String,
    pub(super) prose: Option<groove_ui_kit::markdown::Row>,
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
                    body: String::new(),
                    prose: Some(prose),
                }
            }
            None => Words::default(),
        },
        Slot::Typed { row } => match ui.session.noting.as_ref() {
            Some(noting) => Words {
                author: if row == 0 {
                    BY_USER.to_string()
                } else {
                    String::new()
                },
                lines: if row == 0 {
                    lines(&noting.anchor)
                } else {
                    String::new()
                },
                body: noting
                    .field
                    .shown()
                    .split('\n')
                    .nth(row)
                    .unwrap_or_default()
                    .to_string(),
                prose: None,
            },
            None => Words::default(),
        },
        Slot::Acts { .. } | Slot::Code(_) => Words::default(),
    }
}
