//! Keys and clicks to commands. With the agent pane focused every key is the agent's,
//! except the `ctrl+shift` chords, which are Groove's everywhere. A click goes through
//! what the last frame drew.

use groove_controllers::{AppState, Command, agent, session, workspace};
use groove_types::WorktreeId;

use crate::ctx::Metrics;
use crate::hit::{Cursor, Hits, Target};
use crate::layout::{Edge, Layout};
use crate::palette::{Action, Flow, Palette};
use crate::tokens::Tokens;
use crate::widget::code_at;
use crate::{Drag, Focus, Ui};

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
        Input::Key { key, mods } => key_input(key, mods, ui, app),
        Input::Press { x, y } => press(x, y, ui, app, hits, metrics),
        Input::Move { x, .. } => {
            drag_to(ui, x, metrics);
            Vec::new()
        }
        Input::Release => {
            ui.drag = None;
            Vec::new()
        }
        Input::Scroll { x, delta, .. } => {
            scroll(x, delta, ui, metrics);
            Vec::new()
        }
    }
}

/// The column under the pointer scrolls. Wheel down is rows up; the view clamps the
/// far end.
fn scroll(x: f32, delta: Delta, ui: &mut Ui, metrics: Metrics) {
    let layout = Layout::of(metrics, ui);
    let pixels = match delta {
        Delta::Lines(lines) => lines * Tokens::new(metrics.scale).row,
        Delta::Pixels(pixels) => pixels,
    };
    if x <= layout.rail.right() {
        ui.rail.scroll = (ui.rail.scroll - pixels).max(0.0);
        return;
    }
    if !layout.sidebar.is_empty() && x >= layout.sidebar.x {
        ui.session.files = (ui.session.files - pixels).max(0.0);
        return;
    }
    if x >= layout.workspace.x {
        ui.session.diff = (ui.session.diff - pixels).max(0.0);
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

fn key_input(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
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
        Focus::Rail => Vec::new(),
    }
}

/// A press on a boundary takes hold of it; anywhere else is a click.
fn press(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    if let Some(Target::Split(edge)) = hits.at(x, y) {
        grab(ui, edge, x, metrics);
        return Vec::new();
    }
    click(x, y, ui, app, hits, metrics)
}

/// Takes hold of `edge`, keeping how far from it the pointer landed.
fn grab(ui: &mut Ui, edge: Edge, x: f32, metrics: Metrics) {
    let at = ui.split.edge_at(edge, width_of(metrics), sidebar(ui));
    ui.drag = Some(Drag {
        edge,
        offset: logical(x, metrics) - at,
    });
}

/// The boundary follows the pointer.
fn drag_to(ui: &mut Ui, x: f32, metrics: Metrics) {
    let Some(drag) = ui.drag else {
        return;
    };
    let at = logical(x, metrics) - drag.offset;
    ui.split.drag(drag.edge, at, width_of(metrics), sidebar(ui));
}

fn sidebar(ui: &Ui) -> bool {
    ui.session.sidebar()
}

/// The window's width in logical pixels.
fn width_of(metrics: Metrics) -> f32 {
    logical(metrics.size.rect().w, metrics)
}

fn logical(value: f32, metrics: Metrics) -> f32 {
    value / metrics.scale
}

/// What was drawn under the point, acted on. Anywhere else closes the palette.
fn click(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let target = hits.at(x, y);
    let inside = matches!(target, Some(Target::Palette | Target::PaletteRow(_)));
    if ui.palette.is_some() && !inside {
        ui.palette = None;
        return Vec::new();
    }
    match target {
        Some(Target::Session(session)) => {
            vec![Command::Session(session::Command::Select { session })]
        }
        Some(Target::Tab(tab)) => {
            ui.session.tab = tab;
            Vec::new()
        }
        Some(Target::Fold) => {
            ui.session.folded = !ui.session.folded;
            Vec::new()
        }
        Some(Target::Picker) => selector(ui, app),
        Some(Target::Worktree(worktree)) => select_worktree(app, worktree),
        Some(Target::File(path)) => {
            vec![Command::Workspace(workspace::Command::OpenFile { path })]
        }
        Some(Target::View(view)) => {
            ui.session.view = view;
            Vec::new()
        }
        Some(Target::Code) => {
            ui.session.at = caret(ui, hits, metrics, (x, y));
            Vec::new()
        }
        Some(Target::PaletteRow(at)) => palette_row(at, ui, app),
        Some(Target::Palette | Target::Split(_)) | None => Vec::new(),
    }
}

/// The row and column a click lands on in the open file.
fn caret(ui: &Ui, hits: &Hits, metrics: Metrics, point: (f32, f32)) -> Option<(usize, usize)> {
    let rect = hits.rect_of(&Target::Code)?;
    let tokens = Tokens::new(metrics.scale);
    code_at(&tokens, metrics.cell, rect, ui.session.diff, point)
}

/// Either picker opens the worktree selector.
fn selector(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let flow = Flow::new(Action::SelectWorktree, session);
    let commands = flow.refresh(app).into_iter().collect();
    ui.palette = Some(Palette {
        flow: Some(flow),
        ..Palette::default()
    });
    commands
}

fn select_worktree(app: &AppState, worktree: WorktreeId) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::SelectWorktree {
        session,
        worktree,
    })]
}

/// A click on a row is that row selected, then confirmed.
fn palette_row(at: usize, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(palette) = &mut ui.palette else {
        return Vec::new();
    };
    palette.selected = at;
    let outcome = palette.key(Key::Enter, app);
    if outcome.close {
        ui.palette = None;
    }
    outcome.commands
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
