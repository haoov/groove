//! Keys and clicks to commands. With the agent pane focused every key is the agent's,
//! except the `ctrl+shift` chords, which are Groove's everywhere. A click goes through
//! what the last frame drew.

mod keys;
mod pointer;
mod scroll;

use groove_controllers::{AppState, Command, agent, workspace};

use crate::ctx::Metrics;
use crate::hit::{Cursor, Hits, Target};
use crate::{Focus, Surface, Ui};

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

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Key {
        key: Key,
        mods: Modifiers,
    },
    /// The left button went down here.
    Press {
        x: f32,
        y: f32,
        mods: Modifiers,
    },
    /// The pointer moved here while the button is down.
    Move {
        x: f32,
        y: f32,
    },
    Release,
    /// What the clipboard holds, for whatever has the keyboard.
    Paste(String),
    /// The right button went down here.
    Menu {
        x: f32,
        y: f32,
    },
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
    Lines { across: f32, down: f32 },
    Pixels { across: f32, down: f32 },
}

impl Delta {
    /// How far down it carries, in pixels, with `line` the height of one line.
    pub fn down(self, line: f32) -> f32 {
        match self {
            Delta::Lines { down, .. } => down * line,
            Delta::Pixels { down, .. } => down,
        }
    }

    /// How far across it carries, in pixels, with `step` the width of one.
    pub fn across(self, step: f32) -> f32 {
        match self {
            Delta::Lines { across, .. } => across * step,
            Delta::Pixels { across, .. } => across,
        }
    }
}

/// The clipboard into whatever holds the keyboard: a field here, a buffer by command.
fn pasted(text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    if let Some(palette) = ui.palette.as_mut() {
        palette
            .query
            .push_str(&crate::widget::Field::one_line(text));
        palette.selected = 0;
        return Vec::new();
    }
    if let Some(noting) = ui.session.noting.as_mut() {
        noting.field.paste(text);
        return Vec::new();
    }
    if let Some(term) = ui.session.bar.typing {
        ui.session.bar.of(term).paste(text);
        return Vec::new();
    }
    if let Some(find) = ui.session.find.as_mut().filter(|find| find.typing) {
        find.query.paste(text);
        return Vec::new();
    }
    if ui.board.typing {
        ui.board.filter.paste(text);
        ui.board.offer = 0;
        return Vec::new();
    }
    if ui.session.composing {
        let edit = groove_types::Edit::Insert(text.to_string());
        return vec![Command::Workspace(workspace::Command::Message(edit))];
    }
    if ui.focus == Focus::Agent && ui.showing(app) == Surface::Session {
        return typed_at_agent(text, app);
    }
    vec![Command::Workspace(workspace::Command::Paste)]
}

/// The clipboard at the agent of the open session.
fn typed_at_agent(text: &str, app: &AppState) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    vec![Command::Agent(agent::Command::Paste {
        session,
        text: text.to_string(),
    })]
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
        Input::Press { x, y, mods } => pointer::press(x, y, mods, ui, app, hits, metrics),
        Input::Menu { x, y } => {
            pointer::asked(x, y, ui, app, hits, metrics);
            Vec::new()
        }
        Input::Move { x, y } => pointer::moved(x, y, ui, app, hits, metrics),
        Input::Paste(text) => pasted(&text, ui, app),
        Input::Release => {
            ui.drag = None;
            ui.selecting = false;
            ui.mapping = false;
            let selecting = std::mem::take(&mut ui.agent.selecting);
            let clicked = std::mem::take(&mut ui.agent.clicking);
            let mut out = pointer::dropped(ui, app);
            if clicked {
                out.extend(pointer::agent_released(ui, app, metrics));
            }
            if selecting {
                out.extend(pointer::agent_copied(app));
            }
            out
        }
        Input::Scroll { x, y, delta } => scroll::scroll((x, y), delta, ui, app, hits, metrics),
    }
}

/// The row under the pointer. True when it changed, and the window must redraw.
pub fn hover(ui: &mut Ui, hits: &Hits, x: f32, y: f32) -> bool {
    let at = hits.at(x, y);
    let moved = matches!(at, Some(Target::Bar(_)));
    let changed = at != ui.hover || (moved && (x, y) != ui.at);
    ui.hover = at;
    ui.at = (x, y);
    changed
}

/// The pointer: a drag in flight owns it, else whatever was drawn under it.
pub fn cursor(ui: &Ui, hits: &Hits, x: f32, y: f32) -> Cursor {
    match ui.drag.map(|drag| drag.edge.upright()) {
        Some(true) => Cursor::ColResize,
        Some(false) => Cursor::RowResize,
        None => hits.cursor_at(x, y),
    }
}
