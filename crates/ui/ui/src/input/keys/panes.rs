//! The keys of the panes that are not a bar: the code, the sidebar, the box, the rail.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::{Edit, Motion};

use super::super::{Key, Modifiers};
use crate::editor::Editing;
use crate::keymap::{Action, Keymap};
use crate::{Focus, Ui};

/// The open buffer takes the keystroke: a motion, a change, or a save.
pub(super) fn in_file(
    key: Key,
    mods: Modifiers,
    (ui, app): (&Ui, &AppState),
    keymap: &Keymap,
) -> Vec<Command> {
    let Some(editing) = Editing::keyed(app, ui) else {
        return Vec::new();
    };
    if mods.ctrl || mods.alt {
        return bound(&editing, keymap, key, mods).into_iter().collect();
    }
    let edit = match key {
        Key::Char(c) if !mods.alt => Edit::Insert(c.to_string()),
        Key::Enter => Edit::Newline,
        Key::Tab => Edit::Indent,
        Key::Backspace => Edit::Backspace,
        Key::Delete => Edit::Delete,
        _ => return moved(&editing, key, mods).into_iter().collect(),
    };
    vec![editing.edit(edit)]
}

/// A motion, extending what the caret holds while shift is down.
fn moved(editing: &Editing, key: Key, mods: Modifiers) -> Option<Command> {
    let motion = match key {
        Key::Left => Motion::Left,
        Key::Right => Motion::Right,
        Key::Up => Motion::Up,
        Key::Down => Motion::Down,
        Key::Home => Motion::LineStart,
        Key::End => Motion::LineEnd,
        _ => return None,
    };
    let edit = match mods.shift {
        true => Edit::Extend(motion),
        false => Edit::Move(motion),
    };
    Some(editing.edit(edit))
}

/// What a bound chord asks of the buffer.
fn bound(editing: &Editing, keymap: &Keymap, key: Key, mods: Modifiers) -> Option<Command> {
    let is = |action| keymap.is(action, key, mods);
    match () {
        _ if is(Action::Save) => editing.save(),
        _ if is(Action::Undo) => Some(editing.edit(Edit::Undo)),
        _ if is(Action::Redo) => Some(editing.edit(Edit::Redo)),
        _ if is(Action::SelectAll) => Some(editing.edit(Edit::SelectAll)),
        _ if is(Action::Copy) => Some(editing.copy(false)),
        _ if is(Action::Cut) => Some(editing.copy(true)),
        _ if is(Action::Paste) => Some(editing.paste()),
        _ => None,
    }
}

/// The list, or the commit message once the box has been clicked.
pub(super) fn in_sidebar(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
) -> Vec<Command> {
    if !ui.session.composing {
        return in_list(key, ui, app);
    }
    if key == Key::Escape {
        ui.session.composing = false;
        return Vec::new();
    }
    if keymap.is(Action::Commit, key, mods) {
        ui.session.composing = false;
        return vec![Command::Workspace(workspace::Command::Commit)];
    }
    in_message(key, mods)
}

/// What a keystroke asks of the message.
fn in_message(key: Key, mods: Modifiers) -> Vec<Command> {
    if mods.ctrl {
        return Vec::new();
    }
    let edit = match key {
        Key::Char(c) if !mods.alt => Edit::Insert(c.to_string()),
        Key::Enter => Edit::Newline,
        Key::Backspace => Edit::Backspace,
        Key::Delete => Edit::Delete,
        Key::Left => Edit::Move(Motion::Left),
        Key::Right => Edit::Move(Motion::Right),
        Key::Up => Edit::Move(Motion::Up),
        Key::Down => Edit::Move(Motion::Down),
        Key::Home => Edit::Move(Motion::LineStart),
        Key::End => Edit::Move(Motion::LineEnd),
        _ => return Vec::new(),
    };
    vec![Command::Workspace(workspace::Command::Message(edit))]
}

/// Up and down open the file above or below in the list.
fn in_list(key: Key, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if key == Key::Enter {
        ui.focus = Focus::Workspace;
        return Vec::new();
    }
    let files = crate::views::session::changed(app);
    let active = app.workspace.active().map(|open| &open.path);
    let at = files.iter().position(|file| Some(&file.path) == active);
    let Some(file) = stepped(key, at, files.len()).and_then(|next| files.get(next)) else {
        return Vec::new();
    };
    vec![Command::Workspace(workspace::Command::OpenFile {
        at: None,
        path: file.path.clone(),
    })]
}

/// Up and down move along the sessions the rail shows.
pub(super) fn in_rail(key: Key, ui: &Ui, app: &AppState) -> Vec<Command> {
    let shown = crate::views::rail::shown(app, ui);
    let selected = app.session.selected.as_ref();
    let at = shown
        .iter()
        .position(|open| Some(&open.session.id) == selected);
    let Some(open) = stepped(key, at, shown.len()).and_then(|next| shown.get(next)) else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::Select {
        session: open.session.id.clone(),
    })]
}

/// The row up or down from `at` in a list of `len`, the first one when none is held.
fn stepped(key: Key, at: Option<usize>, len: usize) -> Option<usize> {
    match (key, at) {
        (Key::Up, Some(at)) => Some(at.saturating_sub(1)),
        (Key::Down, Some(at)) => Some((at + 1).min(len.saturating_sub(1))),
        (Key::Up | Key::Down, None) => Some(0),
        _ => None,
    }
}
