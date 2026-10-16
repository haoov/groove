//! The keys of the panes that are not a bar: the code, the sidebar, the box, the rail.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::{Edit, Motion};

use super::super::{Key, Modifiers};
use super::worded::{Asked, asked};
use crate::Ui;
use crate::editor::Editing;
use crate::keymap::{Action, Keymap};

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
        _ => {
            let wrapped = matches!(key, Key::Up | Key::Down)
                .then(|| along(app, ui, key == Key::Down))
                .flatten();
            return moved(&editing, (key, mods), wrapped).into_iter().collect();
        }
    };
    vec![editing.edit(edit)]
}

/// Up or down a row of a wrapped line rather than a whole line; `None` while nothing wraps.
fn along(app: &AppState, ui: &Ui, down: bool) -> Option<Motion> {
    use crate::views::session::diff::{edited, text_at, wrapping};
    use groove_controllers::workspace_service::{column_of, display_of};
    let (starts, cols) = wrapping::wrap_of(app, ui)?;
    let editor = edited(app, ui)?;
    let caret = editor.buffer.caret();
    let (text, width) = text_at((app, ui), &editor.path, caret.line)?;
    let display = display_of(&text, caret.column, width);
    let (line, at) = wrapping::stepped(&starts, cols, (caret.line, display), down)?;
    let (text, width) = text_at((app, ui), &editor.path, line)?;
    Some(Motion::To(groove_types::Caret::new(
        line,
        column_of(&text, at, width),
    )))
}

/// A motion, extending what the caret holds while shift is down.
fn moved(
    editing: &Editing,
    (key, mods): (Key, Modifiers),
    wrapped: Option<Motion>,
) -> Option<Command> {
    let motion = match key {
        Key::Up | Key::Down if wrapped.is_some() => wrapped?,
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
        _ if is(Action::Paste) => editing.paste(),
        _ => None,
    }
}

/// The list, or the commit message once the box has been clicked.
pub(super) fn in_sidebar(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
    seen: (&crate::hit::Hits, groove_ui_kit::base::ctx::Metrics),
) -> Vec<Command> {
    if ui.session.tab == crate::views::session::Tab::Resources {
        return super::resources::in_kinds_list((key, mods), ui, app, seen);
    }
    if !(ui.session.composing && ui.session.commits()) {
        return super::listed::in_list(key, ui, app, seen);
    }
    if key == Key::Escape {
        ui.session.composing = false;
        return Vec::new();
    }
    if keymap.is(Action::Commit, key, mods) {
        ui.session.composing = false;
        return vec![Command::Workspace(workspace::Command::Commit)];
    }
    in_message(key, mods, app, keymap)
}

/// What a keystroke asks of the message: a line, or what a note's buffer answers.
fn in_message(key: Key, mods: Modifiers, app: &AppState, keymap: &Keymap) -> Vec<Command> {
    let asked = match key {
        Key::Enter if !mods.ctrl => Asked {
            edit: Some(Edit::Newline),
            copied: None,
        },
        _ => asked(&app.workspace.message, keymap, key, mods),
    };
    let edit = asked
        .edit
        .map(|edit| Command::Workspace(workspace::Command::Message(edit)));
    asked.copied.into_iter().chain(edit).collect()
}

/// Up and down select a session, Enter goes to its workspace, `a` and `v` answer its ask.
pub(super) fn in_rail(key: Key, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let ask = app
        .session
        .selected
        .as_ref()
        .and_then(|session| app.agent.activity(session))
        .and_then(|one| one.asks.first());
    match (key, ask) {
        (Key::Enter, _) => {
            ui.focus = crate::Focus::Workspace;
            return Vec::new();
        }
        (Key::Char('a'), Some(ask)) => {
            let approve = groove_controllers::agent::Command::Approve { id: ask.id.clone() };
            return vec![Command::Agent(approve)];
        }
        (Key::Char('v'), Some(ask)) => {
            ui.overlay = Some(crate::Overlay::Examining(ask.id.clone()));
            return Vec::new();
        }
        _ => {}
    }
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
pub(super) fn stepped(key: Key, at: Option<usize>, len: usize) -> Option<usize> {
    match (key, at) {
        (Key::Up, Some(at)) => Some(at.saturating_sub(1)),
        (Key::Down, Some(at)) => Some((at + 1).min(len.saturating_sub(1))),
        (Key::Up | Key::Down, None) => Some(0),
        _ => None,
    }
}
