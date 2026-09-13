use groove_controllers::{AppState, Command};

use crate::Ui;

/// A key as the ui reads it, free of the window library's types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Escape,
    Enter,
    Tab,
    Up,
    Down,
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
pub fn handle(input: Input, ui: &mut Ui, _app: &AppState) -> Option<Command> {
    match input {
        Input::Key {
            key: Key::Char('k' | 'p'),
            mods,
        } if mods.ctrl => {
            ui.palette_open = !ui.palette_open;
            None
        }
        Input::Key {
            key: Key::Escape, ..
        } if ui.palette_open => {
            ui.palette_open = false;
            None
        }
        _ => None,
    }
}
