//! Keys and clicks to commands. With the agent pane focused every key is the agent's,
//! except the `ctrl+shift` chords, which are Groove's everywhere. A click goes through
//! what the last frame drew.

mod keys;
mod pointer;
mod scroll;

use groove_controllers::{AppState, Command};

use crate::Ui;
use crate::ctx::Metrics;
use crate::hit::{Cursor, Hits};

pub use keys::encode;

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
    Key {
        key: Key,
        mods: Modifiers,
    },
    /// The left button went down here.
    Press {
        x: f32,
        y: f32,
    },
    /// The pointer moved here while the button is down.
    Move {
        x: f32,
        y: f32,
    },
    Release,
    /// The wheel or the trackpad, over this point.
    Scroll {
        x: f32,
        y: f32,
        delta: Delta,
    },
}

/// What a wheel reports: whole lines, or pixels from a trackpad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Delta {
    Lines(f32),
    Pixels(f32),
}

/// Mutates the ui's own state on the spot; returns the commands a domain action needs.
pub fn handle(
    input: Input,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    match input {
        Input::Key { key, mods } => keys::key_input(key, mods, ui, app),
        Input::Press { x, y } => pointer::press(x, y, ui, app, hits, metrics),
        Input::Move { x, y } => pointer::moved(x, y, ui, app, hits, metrics),
        Input::Release => {
            ui.drag = None;
            ui.selecting = false;
            Vec::new()
        }
        Input::Scroll { x, delta, .. } => {
            scroll::scroll(x, delta, ui, hits, metrics);
            Vec::new()
        }
    }
}

/// The row under the pointer. True when it changed, and the window must redraw.
pub fn hover(ui: &mut Ui, hits: &Hits, x: f32, y: f32) -> bool {
    let at = hits.at(x, y);
    let changed = at != ui.hover;
    ui.hover = at;
    changed
}

/// The pointer: a drag in flight owns it, else whatever was drawn under it.
pub fn cursor(ui: &Ui, hits: &Hits, x: f32, y: f32) -> Cursor {
    match ui.drag {
        Some(_) => Cursor::ColResize,
        None => hits.cursor_at(x, y),
    }
}
