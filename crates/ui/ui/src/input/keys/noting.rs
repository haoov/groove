//! The note being typed in the surface: what it leaves, and what drops it.

use groove_controllers::Command;
use groove_types::Edit;

use super::super::{Key, Modifiers};
use super::worded::asked_of_note;
use crate::Ui;
use crate::keymap::Keymap;

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
        Key::Enter => return sent(ui),
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

/// The note being typed, sent where its words go, and the row closed.
pub(crate) fn sent(ui: &mut Ui) -> Vec<Command> {
    let act = ui.session.noting.take().and_then(|noting| noting.sent());
    act.map(Command::Delivery).into_iter().collect()
}
