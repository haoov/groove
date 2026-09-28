//! Keys and clicks to commands; the agent pane takes every key but Groove's `ctrl+shift` chords.

mod keys;
mod pointer;
mod scroll;

use groove_controllers::{AppState, Command, agent, workspace};

use crate::hit::{Cursor, Hits, Target};
use crate::{Focus, Held, Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;

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
    if let Some(palette) = ui.palette_mut() {
        palette
            .query
            .push_str(&groove_ui_kit::widgets::Field::one_line(text));
        palette.selected = 0;
        return Vec::new();
    }
    if ui.settings.open {
        return pasted_in_settings(text, ui, app);
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
    if ui.focus == Focus::Terminal && ui.showing(app) == Surface::Session {
        return typed_at_shell(text, app);
    }
    vec![Command::Workspace(workspace::Command::Paste)]
}

/// The clipboard in the search while typing, else at a running sign-in.
fn pasted_in_settings(text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if ui.settings.typing {
        ui.settings.search.paste(text);
        return Vec::new();
    }
    if app.agent.login.is_none() {
        return Vec::new();
    }
    let paste = groove_controllers::config::Command::PasteLogin {
        text: text.to_string(),
    };
    vec![Command::Config(paste)]
}

/// The clipboard at the open session's selected terminal.
fn typed_at_shell(text: &str, app: &AppState) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let Some(id) = app.shell.shells(&session).and_then(|one| one.focused()) else {
        return Vec::new();
    };
    let text = text.to_string();
    vec![Command::Shell(groove_controllers::shell::Command::Paste {
        session,
        id,
        text,
    })]
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
            let mut out = pointer::dropped(ui, app);
            match ui.held.take() {
                Some(Held::AgentClick) => out.extend(pointer::agent_released(ui, app, metrics)),
                Some(Held::AgentText) => out.extend(pointer::agent_copied(app)),
                _ => {}
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
    match &ui.held {
        Some(Held::Edge(drag)) if drag.edge.upright() => Cursor::ColResize,
        Some(Held::Edge(_)) => Cursor::RowResize,
        _ => hits.cursor_at(x, y),
    }
}
