//! Keys and clicks to commands; the agent pane takes every key but the app ring, copy, file search and paste.

pub(crate) mod follow;
mod keys;
mod paste;
mod pointer;
mod rest;
mod scroll;
mod selection;
mod typed;

use groove_controllers::{AppState, Command};

use crate::hit::{Cursor, Hits};
use crate::{Held, Ui};
use groove_ui_kit::base::ctx::Metrics;

pub use follow::follow;
pub use keys::encode;
pub use paste::{Aim, aim};
pub use rest::{rest, resting};

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
    /// The middle button went down here.
    Middle {
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

/// Mutates the ui's own state on the spot; returns the commands a domain action needs.
pub fn handle(
    input: Input,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    if matches!(
        input,
        Input::Key { .. } | Input::Press { .. } | Input::Menu { .. } | Input::Paste(_)
    ) {
        ui.settings.offered = true;
    }
    match input {
        Input::Key { key, mods } => keys::key_input(key, mods, ui, app, (hits, metrics)),
        Input::Press { x, y, mods } => pointer::press(x, y, mods, ui, app, hits, metrics),
        Input::Menu { x, y } => {
            pointer::asked(x, y, ui, app, hits, metrics);
            Vec::new()
        }
        Input::Middle { x, y } => pointer::middle(x, y, ui, app, hits),
        Input::Move { x, y } => pointer::moved(x, y, ui, app, hits, metrics),
        Input::Paste(text) => paste::pasted(&text, ui, app, (hits, metrics)),
        Input::Release => {
            let mut out = pointer::dropped(ui, app);
            match ui.held.take() {
                Some(Held::Clicking(grid)) => {
                    out.extend(selection::let_go(ui, app, grid, hits, metrics))
                }
                Some(Held::Selecting(grid)) => out.extend(selection::copied(app, grid)),
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
    let changed = at != ui.hover;
    ui.hover = at;
    ui.at = (x, y);
    changed
}

/// The pointer: a drag in flight owns it, else whatever was drawn under it.
pub fn cursor(ui: &Ui, hits: &Hits, x: f32, y: f32) -> Cursor {
    match &ui.held {
        Some(Held::Edge(drag)) if drag.edge.upright() => Cursor::ColResize,
        Some(Held::Edge(_)) => Cursor::RowResize,
        Some(Held::Column(_)) => Cursor::ColResize,
        _ => hits.cursor_at(x, y),
    }
}

/// What a palette row asks of the window, done as its chord or its click would.
pub(crate) fn acted(act: crate::palette::Act, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    use crate::palette::Act;
    use crate::views::session::{Noting, Pane, Tab};
    match act {
        Act::Key(action) => return keys::run(action, ui, app),
        Act::View(view) => {
            ui.session.view = view;
            ui.session.tab = Tab::Diff;
        }
        Act::Note => {
            let Some(open) = app.workspace.active() else {
                return Vec::new();
            };
            let line = open.new.caret().line as u32;
            let anchor = groove_types::Anchor::line(open.path.clone(), line);
            ui.session.noting = Some(Noting::new(anchor));
            ui.focus = crate::Focus::Workspace;
        }
        Act::Notes(showing) => {
            ui.session.pane = Pane::Notes;
            ui.session.showing = showing;
            ui.session.folded = false;
        }
        Act::Examine(id) => ui.overlay = Some(crate::Overlay::Examining(id)),
        Act::Discard => ui.overlay = Some(crate::Overlay::Losing(crate::Losing::Everything)),
        Act::Open(path) => {
            ui.session.tab = Tab::Files;
            ui.focus = crate::Focus::Workspace;
            let open = groove_controllers::workspace::Command::OpenFile { path, at: None };
            return vec![Command::Workspace(open)];
        }
    }
    Vec::new()
}
