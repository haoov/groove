//! The keys of the panes that are not a bar: the code, the sidebar, the box, the rail.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::{Edit, Motion};

use super::super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::{Focus, Ui};

/// The open buffer takes the keystroke: a motion, a change, or a save.
pub(super) fn in_file(key: Key, mods: Modifiers, app: &AppState, keymap: &Keymap) -> Vec<Command> {
    if app.workspace.active().is_none() {
        return Vec::new();
    }
    if mods.ctrl || mods.alt {
        return bound(keymap, key, mods).into_iter().collect();
    }
    let edit = match key {
        Key::Char(c) if !mods.alt => Edit::Insert(c.to_string()),
        Key::Enter => Edit::Newline,
        Key::Tab => Edit::Indent,
        Key::Backspace => Edit::Backspace,
        Key::Delete => Edit::Delete,
        _ => return moved(key, mods).into_iter().collect(),
    };
    vec![Command::Workspace(workspace::Command::Edit(edit))]
}

/// A motion, extending what the caret holds while shift is down.
fn moved(key: Key, mods: Modifiers) -> Option<Command> {
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
    Some(Command::Workspace(workspace::Command::Edit(edit)))
}

/// What a bound chord asks of the buffer.
fn bound(keymap: &Keymap, key: Key, mods: Modifiers) -> Option<Command> {
    let is = |action| keymap.is(action, key, mods);
    let command = match () {
        _ if is(Action::Save) => workspace::Command::SaveFile,
        _ if is(Action::Undo) => workspace::Command::Edit(Edit::Undo),
        _ if is(Action::Redo) => workspace::Command::Edit(Edit::Redo),
        _ if is(Action::SelectAll) => workspace::Command::Edit(Edit::SelectAll),
        _ if is(Action::Copy) => workspace::Command::Copy,
        _ if is(Action::Cut) => workspace::Command::Cut,
        _ if is(Action::Paste) => workspace::Command::Paste,
        _ => return None,
    };
    Some(Command::Workspace(command))
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
    let files = crate::views::session::changed(app);
    let at = files
        .iter()
        .position(|file| Some(&file.path) == app.workspace.active().map(|o| &o.path));
    let next = match (key, at) {
        (Key::Up, Some(at)) => at.saturating_sub(1),
        (Key::Down, Some(at)) => (at + 1).min(files.len().saturating_sub(1)),
        (Key::Up | Key::Down, None) => 0,
        (Key::Enter, _) => {
            ui.focus = Focus::Workspace;
            return Vec::new();
        }
        _ => return Vec::new(),
    };
    let Some(file) = files.get(next) else {
        return Vec::new();
    };
    vec![Command::Workspace(workspace::Command::OpenFile {
        at: None,
        path: file.path.clone(),
    })]
}

/// Up and down move along the opened sessions.
pub(super) fn in_rail(key: Key, app: &AppState) -> Vec<Command> {
    let at = app
        .session
        .open
        .iter()
        .position(|open| Some(&open.session.id) == app.session.selected.as_ref());
    let next = match (key, at) {
        (Key::Up, Some(at)) => at.saturating_sub(1),
        (Key::Down, Some(at)) => (at + 1).min(app.session.open.len().saturating_sub(1)),
        (Key::Up | Key::Down, None) => 0,
        _ => return Vec::new(),
    };
    let Some(open) = app.session.open.get(next) else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::Select {
        session: open.session.id.clone(),
    })]
}
