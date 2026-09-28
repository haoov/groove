//! What a key does, by the pane that holds the keyboard.

mod agent;
mod app;
mod bar;
mod filter;
mod naming;
mod noting;
mod panes;

use groove_controllers::{AppState, Command};

use super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::views::session::Term;
use crate::views::settings::Draft;
use crate::{Focus, Surface, Ui};

pub use agent::encode;
use agent::{to_agent, to_shell};
use bar::{finding, in_bar, opened, typing};
use filter::on_board;
use naming::in_name;
use noting::in_note;
use panes::{in_file, in_rail, in_sidebar};

pub(super) fn key_input(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
    seen: (&crate::hit::Hits, groove_ui_kit::base::ctx::Metrics),
) -> Vec<Command> {
    let keymap = Keymap::of(app.config.config.as_ref());
    if let Some(action) = ui.settings.binding {
        return bound(action, key, mods, ui, app);
    }
    if (ui.examining().is_some() || ui.menu().is_some()) && key == Key::Escape {
        ui.overlay = None;
        return Vec::new();
    }
    if let Some(action) = keymap.app(key, mods) {
        return app::run(action, ui, app);
    }
    if let Some(palette) = ui.palette_mut() {
        let outcome = palette.key(key, app);
        return ui.closed_palette(outcome);
    }
    if ui.settings.open {
        return in_settings(key, mods, ui, app);
    }
    if ui.session.naming.is_some() {
        return in_name(key, mods, ui);
    }
    if ui.session.noting.is_some() {
        return in_note(key, mods, ui);
    }
    if ui.session.bar.typing.is_some() {
        return in_bar(key, mods, ui, app, &keymap);
    }
    if let Some(commands) = finding((key, mods), ui, app, &keymap, seen) {
        return commands;
    }
    in_pane(key, mods, ui, app, &keymap)
}

/// No bar holds the keyboard: the pane that has it takes the key, after the chords it binds.
fn in_pane(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
) -> Vec<Command> {
    let raw = matches!(ui.focus, Focus::Agent | Focus::Terminal);
    if raw && keymap.is(Action::TerminalCopy, key, mods) {
        return app::copied(app);
    }
    let bars = [
        (Action::OpenPath, Term::Path, !raw),
        (Action::SearchFiles, Term::Text, true),
    ];
    for (action, term, heard) in bars {
        if heard && keymap.is(action, key, mods) {
            opened(ui, app, term);
            return Vec::new();
        }
    }
    if ui.showing(app) == Surface::Board {
        return on_board(key, mods, ui, app);
    }
    match ui.focus {
        Focus::Agent => to_agent(key, mods, app).into_iter().collect(),
        Focus::Terminal => to_shell(key, mods, app).into_iter().collect(),
        Focus::Workspace => in_file(key, mods, app, keymap),
        Focus::Sidebar => in_sidebar(key, mods, ui, app, keymap),
        Focus::Rail => in_rail(key, app),
    }
}

/// The key pressed as the action's new chord; Esc gives up, and a chord needs ctrl or alt.
fn bound(action: Action, key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if key == Key::Escape && !mods.ctrl && !mods.alt {
        ui.settings.binding = None;
        return Vec::new();
    }
    if !mods.ctrl && !mods.alt {
        return Vec::new();
    }
    ui.settings.binding = None;
    let chord = crate::keymap::chord_of(key, mods);
    let keymap = crate::keymap::rebound(app.config.config.as_ref(), action, chord);
    vec![Command::Config(
        groove_controllers::config::Command::Rebind(keymap),
    )]
}

/// Settings holds the keyboard: the search, a source's field, a running sign-in, else Esc back.
fn in_settings(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let settings = &mut ui.settings;
    let signing_in = app.agent.login.is_some();
    if let Some(draft) = settings.draft.as_mut().filter(|one| one.at.is_some()) {
        return in_draft(key, mods, draft);
    }
    match (key, settings.typing) {
        (Key::Escape | Key::Enter, true) => settings.typing = false,
        (key, true) => {
            typing(key, mods, &mut settings.search);
        }
        (key, false) if signing_in => {
            let Some(bytes) = encode(key, mods) else {
                return Vec::new();
            };
            let send = groove_controllers::config::Command::SendLogin { bytes };
            return vec![Command::Config(send)];
        }
        (Key::Escape, false) => return ui.close_settings(),
        _ => {}
    }
    Vec::new()
}

/// A source's fields: Tab to the next, Enter to connect, Esc to let go of them.
fn in_draft(key: Key, mods: Modifiers, draft: &mut Draft) -> Vec<Command> {
    match key {
        Key::Escape => draft.at = None,
        Key::Tab => draft.next(),
        Key::Enter => {
            draft.at = None;
            return vec![draft.command()];
        }
        key => {
            if let Some(field) = draft.focused() {
                typing(key, mods, field);
            }
        }
    }
    Vec::new()
}
