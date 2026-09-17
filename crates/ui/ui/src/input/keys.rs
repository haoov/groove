//! What a key does, by the pane that holds the keyboard.

use groove_controllers::{AppState, Command, agent, session, workspace};

use groove_types::{Edit, Motion};

use super::{Key, Modifiers};
use crate::palette::Palette;
use crate::{Focus, Ui};

pub(super) fn key_input(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if mods.ctrl && mods.shift {
        return chord(key, ui, app).into_iter().collect();
    }
    if let Some(palette) = &mut ui.palette {
        let outcome = palette.key(key, app);
        if outcome.close {
            ui.palette = None;
        }
        return outcome.commands;
    }
    match ui.focus {
        Focus::Agent => to_agent(key, mods, app).into_iter().collect(),
        Focus::Workspace => in_file(key, mods, app),
        Focus::Sidebar => in_list(key, ui, app),
        Focus::Rail => in_rail(key, app),
    }
}

/// The open buffer takes the keystroke: a motion, a change, or a save.
fn in_file(key: Key, mods: Modifiers, app: &AppState) -> Vec<Command> {
    if app.workspace.opened.is_none() {
        return Vec::new();
    }
    if mods.ctrl {
        return with_ctrl(key).into_iter().collect();
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

/// What the control key asks of the buffer.
fn with_ctrl(key: Key) -> Option<Command> {
    let edit = match key {
        Key::Char('s' | 'S') => return Some(Command::Workspace(workspace::Command::SaveFile)),
        Key::Char('z' | 'Z') => Edit::Undo,
        Key::Char('y' | 'Y') => Edit::Redo,
        Key::Char('a' | 'A') => Edit::SelectAll,
        Key::Char('c' | 'C') => return Some(Command::Workspace(workspace::Command::Copy)),
        Key::Char('x' | 'X') => return Some(Command::Workspace(workspace::Command::Cut)),
        Key::Char('v' | 'V') => return Some(Command::Workspace(workspace::Command::Paste)),
        _ => return None,
    };
    Some(Command::Workspace(workspace::Command::Edit(edit)))
}

/// Up and down open the file above or below in the list.
fn in_list(key: Key, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let files = crate::views::session::changed(app);
    let at = files
        .iter()
        .position(|file| Some(&file.path) == app.workspace.opened.as_ref().map(|o| &o.path));
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
        path: file.path.clone(),
    })]
}

/// Up and down move along the opened sessions.
fn in_rail(key: Key, app: &AppState) -> Vec<Command> {
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

/// Groove's own shortcuts.
fn chord(key: Key, ui: &mut Ui, app: &AppState) -> Option<Command> {
    match key {
        Key::Char('p' | 'P') => {
            if ui.palette.take().is_some() {
                return None;
            }
            ui.palette = Some(Palette::default());
            Some(Command::Session(session::Command::ListRepos))
        }
        Key::Char('n' | 'N') => Some(Command::Session(session::Command::OpenExplorer {
            title: None,
        })),
        Key::Char('b' | 'B') => {
            ui.session.folded = !ui.session.folded;
            None
        }
        Key::Char('r' | 'R') => Some(Command::Workspace(workspace::Command::Load)),
        Key::Left | Key::Right => {
            ui.focus = ui.focus.beside(key == Key::Right);
            None
        }
        Key::Char('w' | 'W') => {
            let session = app.session.selected.clone()?;
            Some(Command::Session(session::Command::Close { session }))
        }
        _ => None,
    }
}

fn to_agent(key: Key, mods: Modifiers, app: &AppState) -> Option<Command> {
    let session = app.session.selected.clone()?;
    let bytes = encode(key, mods)?;
    Some(Command::Agent(agent::Command::Send { session, bytes }))
}

/// The bytes a terminal sends for a key. `None` for a key a terminal has no word for.
pub fn encode(key: Key, mods: Modifiers) -> Option<Vec<u8>> {
    let bytes = match key {
        Key::Char(c) if mods.ctrl => vec![control(c)?],
        Key::Char(c) if mods.alt => {
            let mut b = vec![0x1b];
            b.extend(c.to_string().into_bytes());
            b
        }
        Key::Char(c) => c.to_string().into_bytes(),
        Key::Enter => b"\r".to_vec(),
        Key::Escape => b"\x1b".to_vec(),
        Key::Tab if mods.shift => b"\x1b[Z".to_vec(),
        Key::Tab => b"\t".to_vec(),
        Key::Backspace => b"\x7f".to_vec(),
        Key::Delete => b"\x1b[3~".to_vec(),
        Key::Up => b"\x1b[A".to_vec(),
        Key::Down => b"\x1b[B".to_vec(),
        Key::Right => b"\x1b[C".to_vec(),
        Key::Left => b"\x1b[D".to_vec(),
        Key::Home => b"\x1b[H".to_vec(),
        Key::End => b"\x1b[F".to_vec(),
        Key::PageUp => b"\x1b[5~".to_vec(),
        Key::PageDown => b"\x1b[6~".to_vec(),
    };
    Some(bytes)
}

/// `ctrl+a` is 0x01 … `ctrl+z` is 0x1a; `ctrl+[` `\` `]` `^` `_` follow.
fn control(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    match c {
        'a'..='z' => Some(c as u8 - b'a' + 1),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '-' => Some(0x1f),
        ' ' | '2' | '@' => Some(0x00),
        _ => None,
    }
}
