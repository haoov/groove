//! What a key does, by the pane that holds the keyboard.

mod agent;
mod bar;
mod filter;
mod naming;
mod noting;
mod panes;

use groove_controllers::{AppState, Command, session, workspace};

use super::{Key, Modifiers};
use crate::palette::Palette;
use crate::views::session::Term;
use crate::{Focus, Overlay, Surface, Ui};

pub use agent::encode;
use agent::to_agent;
use bar::{finding, in_bar, opened, typing};
use filter::on_board;
use naming::in_name;
use noting::in_note;
use panes::{in_file, in_rail, in_sidebar};

pub(super) fn key_input(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if mods.ctrl && mods.shift {
        return match key {
            Key::Char('k' | 'K') => board(ui),
            key => chord(key, ui, app).into_iter().collect(),
        };
    }
    if ui.examining().is_some() && key == Key::Escape {
        ui.overlay = None;
        return Vec::new();
    }
    if let Some(palette) = ui.palette_mut() {
        let outcome = palette.key(key, app);
        if outcome.close {
            ui.overlay = None;
        }
        return outcome.commands;
    }
    if ui.session.naming.is_some() {
        return in_name(key, mods, ui);
    }
    if ui.session.noting.is_some() {
        return in_note(key, mods, ui);
    }
    if ui.session.bar.typing.is_some() {
        return in_bar(key, mods, ui, app);
    }
    if let Some(commands) = finding(key, mods, ui, app) {
        return commands;
    }
    if mods.ctrl && matches!(key, Key::Char('p' | 'P')) && ui.focus != Focus::Agent {
        opened(ui, app, Term::Path);
        return Vec::new();
    }
    if ui.showing(app) == Surface::Board {
        return on_board(key, mods, ui, app);
    }
    match ui.focus {
        Focus::Agent => to_agent(key, mods, app).into_iter().collect(),
        Focus::Workspace => in_file(key, mods, app),
        Focus::Sidebar => in_sidebar(key, mods, ui, app),
        Focus::Rail => in_rail(key, app),
    }
}

/// The board, or the session it was opened from.
fn board(ui: &mut Ui) -> Vec<Command> {
    ui.surface = match ui.surface {
        Surface::Board => Surface::Session,
        Surface::Session => Surface::Board,
    };
    match ui.surface {
        Surface::Board => crate::input::pointer::board_reads(),
        Surface::Session => Vec::new(),
    }
}

/// Groove's own shortcuts.
fn chord(key: Key, ui: &mut Ui, app: &AppState) -> Option<Command> {
    match key {
        Key::Char('p' | 'P') => {
            if ui.close(|one| matches!(one, Overlay::Palette(_))).is_some() {
                return None;
            }
            ui.overlay = Some(Overlay::Palette(Palette::default()));
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
        Key::Char('f' | 'F') => {
            opened(ui, app, Term::Text);
            None
        }
        Key::Left | Key::Right => {
            ui.focus = ui.focus.beside(key == Key::Right);
            None
        }
        Key::Char('w' | 'W') => {
            let session = app.session.selected.clone()?;
            Some(Command::Session(session::Command::Close { session }))
        }
        Key::Char('c' | 'C') => {
            let session = app.session.selected.clone()?;
            let copy = groove_controllers::agent::Command::Copy { session };
            Some(Command::Agent(copy))
        }
        _ => None,
    }
}
