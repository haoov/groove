//! What a key does, by the pane that holds the keyboard.

mod agent;
mod app;
mod bar;
mod board;
mod filter;
mod naming;
mod noting;
mod panes;
mod resources;
mod scope;

use groove_controllers::{AppState, Command};

use super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::views::session::Term;
use crate::views::session::resources::Line;
use crate::views::settings::Draft;
use crate::{Focus, Surface, Ui};

pub use agent::encode;
use agent::{to_agent, to_shell};
use bar::{finding, in_bar, opened, typing};
use filter::on_board;
use naming::in_name;
use noting::in_note;
use panes::{in_file, in_rail, in_sidebar};
pub(in crate::input) use scope::pick;

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
    if let Some(commands) = scope::in_scope(key, mods, ui, app) {
        return commands;
    }
    if let Some(palette) = ui.palette_mut() {
        if matches!(key, Key::Char(_)) && (mods.ctrl || mods.alt) {
            return Vec::new();
        }
        let outcome = palette.key(key, app);
        return ui.closed_palette(outcome, app);
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
    if ui.session.resources.typing || ui.session.resources.filtering {
        return in_resources(key, mods, ui, app);
    }
    if let Some(commands) = finding((key, mods), ui, app, &keymap, seen) {
        return commands;
    }
    in_pane((key, mods), ui, app, &keymap, seen)
}

/// A Resources search has the keys: Enter gives them back; Esc too, and shuts an empty find bar.
fn in_resources(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if ui.session.resources.filtering {
        return in_kinds(key, mods, ui, app);
    }
    let held = &mut ui.session.resources;
    match key {
        Key::Enter => held.typing = false,
        Key::Escape => {
            held.finding &= !held.search.is_empty();
            held.typing = false;
        }
        key => {
            typing(key, mods, &mut held.search);
            held.scroll = 0.0;
        }
    }
    Vec::new()
}

/// The kind search: arrows step through the lines; Enter folds a heading or lists a kind.
fn in_kinds(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let shown = crate::views::session::resources::shown(app, ui);
    let held = &mut ui.session.resources;
    let last = shown.len().checked_sub(1);
    match key {
        Key::Down => {
            held.cursor = held
                .cursor
                .map_or(Some(0), |at| Some(at + 1))
                .zip(last)
                .map(|(at, end)| at.min(end))
        }
        Key::Up => held.cursor = held.cursor.map(|at| at.saturating_sub(1)),
        Key::Enter => match held.cursor.and_then(|at| shown.get(at)) {
            Some(Line::Heading(heading, _)) => {
                if !held.folded.remove(heading) {
                    held.folded.insert(heading.clone());
                }
            }
            Some(Line::Kind(kind)) => {
                (held.kind, held.scroll) = (Some(kind.clone()), 0.0);
                (held.filtering, held.cursor) = (false, None);
            }
            None => (held.filtering, held.cursor) = (false, None),
        },
        Key::Escape => (held.filtering, held.cursor) = (false, None),
        key => {
            typing(key, mods, &mut held.filter);
            let lines = crate::views::session::resources::shown(app, ui);
            let first = lines.iter().position(|one| matches!(one, Line::Kind(_)));
            ui.session.resources.cursor = first;
            return Vec::new();
        }
    }
    Vec::new()
}

/// No bar holds the keyboard: the pane that has it takes the key, after the chords it binds.
fn in_pane(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
    seen: (&crate::hit::Hits, groove_ui_kit::base::ctx::Metrics),
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
        return on_board(key, mods, ui, app, seen);
    }
    match ui.focus {
        Focus::Agent => to_agent(key, mods, app).into_iter().collect(),
        Focus::Terminal => to_shell(key, mods, app).into_iter().collect(),
        Focus::Workspace => in_file(key, mods, app, keymap),
        Focus::Sidebar => in_sidebar(key, mods, ui, app, keymap),
        Focus::Rail => in_rail(key, ui, app),
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
