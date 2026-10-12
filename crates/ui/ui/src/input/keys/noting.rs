//! The note being typed in the surface: what it leaves, and what drops it.

use groove_controllers::Command;
use groove_controllers::delivery::{self, NoteAct, ThreadAct};
use groove_types::Edit;

use super::super::{Key, Modifiers};
use super::worded::asked_of_note;
use crate::Ui;
use crate::keymap::Keymap;
use crate::views::session::{Noting, Writing};

/// One keystroke while a note is being typed.
pub(super) fn in_note(key: Key, mods: Modifiers, ui: &mut Ui, keymap: &Keymap) -> Vec<Command> {
    let Some(noting) = ui.session.noting.as_mut() else {
        return Vec::new();
    };
    match key {
        Key::Escape => ui.session.noting = None,
        Key::Enter if mods.shift => {
            noting.buffer.edit(&Edit::Newline);
        }
        Key::Enter => {
            let act = left(noting);
            ui.session.noting = None;
            return act
                .map(|act| vec![Command::Delivery(act)])
                .unwrap_or_default();
        }
        key => {
            let asked = asked_of_note(&noting.buffer, keymap, key, mods);
            if let Some(edit) = asked.edit {
                noting.buffer.edit(&edit);
            }
            return asked.copied.into_iter().collect();
        }
    }
    Vec::new()
}

/// The note its words leave, or nothing while it says nothing.
fn left(noting: &Noting) -> Option<delivery::Command> {
    let said = noting.said();
    if said.is_empty() {
        return None;
    }
    match noting.writing.clone() {
        Writing::Over(id) => Some(delivery::Command::Note(NoteAct::Update {
            id,
            content: said,
        })),
        Writing::Reply(thread) => Some(delivery::Command::Thread(ThreadAct::Reply {
            thread,
            body: said,
        })),
        Writing::New => Some(delivery::Command::Note(NoteAct::Create {
            anchor: noting.anchor.clone(),
            content: said,
        })),
    }
}
