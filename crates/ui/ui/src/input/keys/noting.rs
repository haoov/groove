//! The note being typed in the surface: what it leaves, and what drops it.

use groove_controllers::delivery::{self, NoteAct, ThreadAct};
use groove_controllers::{Command, cluster};
use groove_types::{Edit, Motion};

use super::super::{Key, Modifiers};
use crate::Ui;
use crate::keymap::{Action, Keymap};
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
        _ if mods.ctrl || mods.alt => return bound(noting, keymap, key, mods),
        key => {
            if let Some(edit) = edit_of(key, mods) {
                noting.buffer.edit(&edit);
            }
        }
    }
    Vec::new()
}

/// What a plain keystroke asks of the note's buffer.
fn edit_of(key: Key, mods: Modifiers) -> Option<Edit> {
    let motion = match key {
        Key::Char(c) => return Some(Edit::Insert(c.to_string())),
        Key::Backspace => return Some(Edit::Backspace),
        Key::Delete => return Some(Edit::Delete),
        Key::Left => Motion::Left,
        Key::Right => Motion::Right,
        Key::Up => Motion::Up,
        Key::Down => Motion::Down,
        Key::Home => Motion::LineStart,
        Key::End => Motion::LineEnd,
        _ => return None,
    };
    Some(match mods.shift {
        true => Edit::Extend(motion),
        false => Edit::Move(motion),
    })
}

/// What a bound chord asks of the note: an undo, a selection, or what it holds to the clipboard.
fn bound(noting: &mut Noting, keymap: &Keymap, key: Key, mods: Modifiers) -> Vec<Command> {
    let is = |action| keymap.is(action, key, mods);
    let edit = match () {
        _ if is(Action::Undo) => Edit::Undo,
        _ if is(Action::Redo) => Edit::Redo,
        _ if is(Action::SelectAll) => Edit::SelectAll,
        _ if is(Action::Copy) || is(Action::Cut) => return copied(noting, is(Action::Cut)),
        _ => return Vec::new(),
    };
    noting.buffer.edit(&edit);
    Vec::new()
}

/// What the caret holds, to the clipboard; and out of the note with `cut`.
fn copied(noting: &mut Noting, cut: bool) -> Vec<Command> {
    let text = noting.buffer.selected();
    if text.is_empty() {
        return Vec::new();
    }
    if cut {
        noting.buffer.edit(&Edit::Delete);
    }
    vec![Command::Cluster(cluster::Command::CopyValue { text })]
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
