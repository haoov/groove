//! The keys a message typed on a buffer answers: the note's and the commit box's.

use groove_controllers::workspace_service::Buffer;
use groove_controllers::{Command, cluster};
use groove_types::{Edit, Motion};

use super::super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};

/// What one keystroke asks of a message: a change to its buffer, and what goes to the clipboard.
#[derive(Default)]
pub(super) struct Asked {
    pub(super) edit: Option<Edit>,
    pub(super) copied: Option<Command>,
}

/// One keystroke on `buffer`, other than Enter and Escape.
pub(super) fn asked(buffer: &Buffer, keymap: &Keymap, key: Key, mods: Modifiers) -> Asked {
    if mods.ctrl || mods.alt {
        return bound(buffer, keymap, key, mods);
    }
    Asked {
        edit: edit_of(key, mods),
        copied: None,
    }
}

/// What a plain keystroke asks of the buffer.
fn edit_of(key: Key, mods: Modifiers) -> Option<Edit> {
    match key {
        Key::Char(c) => Some(Edit::Insert(c.to_string())),
        Key::Backspace => Some(Edit::Backspace),
        Key::Delete => Some(Edit::Delete),
        key => motion_of(key).map(|motion| match mods.shift {
            true => Edit::Extend(motion),
            false => Edit::Move(motion),
        }),
    }
}

fn motion_of(key: Key) -> Option<Motion> {
    Some(match key {
        Key::Left => Motion::Left,
        Key::Right => Motion::Right,
        Key::Up => Motion::Up,
        Key::Down => Motion::Down,
        Key::Home => Motion::LineStart,
        Key::End => Motion::LineEnd,
        _ => return None,
    })
}

/// What a chord asks: an undo, a selection, the clipboard, or an alt motion that moves the caret.
fn bound(buffer: &Buffer, keymap: &Keymap, key: Key, mods: Modifiers) -> Asked {
    let is = |action| keymap.is(action, key, mods);
    let edit = match () {
        _ if is(Action::Undo) => Edit::Undo,
        _ if is(Action::Redo) => Edit::Redo,
        _ if is(Action::SelectAll) => Edit::SelectAll,
        _ if is(Action::Copy) || is(Action::Cut) => return copied(buffer, is(Action::Cut)),
        _ => {
            return Asked {
                edit: motion_of(key).filter(|_| !mods.ctrl).map(Edit::Move),
                copied: None,
            };
        }
    };
    Asked {
        edit: Some(edit),
        copied: None,
    }
}

/// What the caret holds, to the clipboard; and out of the buffer with `cut`.
fn copied(buffer: &Buffer, cut: bool) -> Asked {
    let text = buffer.selected();
    if text.is_empty() {
        return Asked::default();
    }
    Asked {
        edit: cut.then_some(Edit::Delete),
        copied: Some(Command::Cluster(cluster::Command::CopyValue { text })),
    }
}
