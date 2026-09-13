//! Keys to commands. With the agent pane focused every key is the agent's, except
//! the `ctrl+shift` chords, which are Groove's everywhere.

use groove_controllers::{AppState, Command, agent, session};

use crate::{Focus, Ui};

/// A key as the ui reads it, free of the window library's types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Escape,
    Enter,
    Tab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Key { key: Key, mods: Modifiers },
    Click { x: f32, y: f32 },
}

/// Mutates the ui's own state on the spot; returns the command a domain action needs.
pub fn handle(input: Input, ui: &mut Ui, app: &AppState) -> Option<Command> {
    let Input::Key { key, mods } = input else {
        return None;
    };
    if mods.ctrl && mods.shift {
        return chord(key, ui, app);
    }
    if ui.palette_open {
        if key == Key::Escape {
            ui.palette_open = false;
        }
        return None;
    }
    match ui.focus {
        Focus::Agent => to_agent(key, mods, app),
        Focus::Rail => None,
    }
}

/// Groove's own shortcuts.
fn chord(key: Key, ui: &mut Ui, app: &AppState) -> Option<Command> {
    match key {
        Key::Char('p' | 'P') => {
            ui.palette_open = !ui.palette_open;
            None
        }
        Key::Char('n' | 'N') => Some(Command::Session(session::Command::OpenExplorer {
            title: None,
        })),
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
