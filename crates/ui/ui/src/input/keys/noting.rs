//! The note being typed in the surface: what it leaves, and what drops it.

use groove_controllers::Command;
use groove_controllers::delivery::{self, NoteAct, ThreadAct};

use super::super::{Key, Modifiers};
use super::typing;
use crate::Ui;
use crate::views::session::Writing;
use crate::views::session::diff::AUTHOR;

/// One keystroke while a note is being typed.
pub(super) fn in_note(key: Key, mods: Modifiers, ui: &mut Ui) -> Vec<Command> {
    let Some(noting) = ui.session.noting.as_mut() else {
        return Vec::new();
    };
    match key {
        Key::Escape => ui.session.noting = None,
        Key::Enter if mods.shift => noting.field.insert('\n'),
        Key::Enter => {
            let act = left(noting);
            ui.session.noting = None;
            return act
                .map(|act| vec![Command::Delivery(act)])
                .unwrap_or_default();
        }
        key => {
            typing(key, mods, &mut noting.field);
        }
    }
    Vec::new()
}

/// The note its words leave, or nothing while it says nothing.
fn left(noting: &crate::views::session::Noting) -> Option<delivery::Command> {
    let said = noting.said();
    if said.is_empty() {
        return None;
    }
    match noting.writing.clone() {
        Writing::Over(id) => Some(delivery::Command::Note(NoteAct::Update {
            id,
            content: said.to_string(),
        })),
        Writing::Reply(thread) => Some(delivery::Command::Thread(ThreadAct::Reply {
            thread,
            body: said.to_string(),
        })),
        Writing::New => Some(delivery::Command::Note(NoteAct::Create {
            anchor: noting.anchor.clone(),
            content: said.to_string(),
            author: AUTHOR.to_string(),
        })),
    }
}
