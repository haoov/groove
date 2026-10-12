//! The clipboard into whatever holds the keyboard, and where a paste would land.

use groove_controllers::{AppState, Command, agent, workspace};

use super::keys;
use super::typed::{self, Typed};
use crate::hit::Hits;
use crate::{Focus, Overlay, Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;

type Seen<'a> = (&'a Hits, Metrics);

/// What holds the keyboard for a paste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sink {
    Scope,
    Palette,
    Settings,
    Board,
    Field(Typed),
    Find,
    Message,
    Agent,
    Terminal,
    Buffer,
}

/// Where a paste would land now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aim(Option<Sink>);

pub fn aim(ui: &Ui, app: &AppState) -> Aim {
    Aim(sink(ui, app))
}

fn sink(ui: &Ui, app: &AppState) -> Option<Sink> {
    if matches!(ui.overlay, Some(Overlay::Scope(_))) {
        return Some(Sink::Scope);
    }
    if ui.palette().is_some() {
        return Some(Sink::Palette);
    }
    if ui.settings.open {
        return Some(Sink::Settings);
    }
    if ui.showing(app) == Surface::Board {
        return ui.board.typing.then_some(Sink::Board);
    }
    if let Some(typed) = typed::keyed(&ui.session) {
        return Some(Sink::Field(typed));
    }
    let session = &ui.session;
    if session.find.as_ref().is_some_and(|find| find.typing) {
        return Some(Sink::Find);
    }
    if session.composing && ui.focus == Focus::Sidebar {
        return Some(Sink::Message);
    }
    Some(match ui.focus {
        Focus::Agent => Sink::Agent,
        Focus::Terminal => Sink::Terminal,
        _ => Sink::Buffer,
    })
}

/// The clipboard into whatever holds the keyboard: a field here, a buffer by command.
pub(super) fn pasted(text: &str, ui: &mut Ui, app: &AppState, seen: Seen) -> Vec<Command> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    match sink(ui, app) {
        Some(Sink::Scope) => {
            if let Some(Overlay::Scope(scoping)) = &mut ui.overlay {
                scoping.query.paste(text);
            }
            keys::scope_changed(ui, app);
        }
        Some(Sink::Palette) => {
            if let Some(palette) = ui.palette_mut() {
                palette.query.paste(text);
                palette.selected = 0;
            }
        }
        Some(Sink::Settings) => return pasted_in_settings(text, ui, app),
        Some(Sink::Board) => {
            ui.board.filter.paste(text);
            ui.board.offer = 0;
        }
        Some(Sink::Field(typed)) => return pasted_in_field(typed, text, ui, app),
        Some(Sink::Find) => return pasted_in_find(text, ui, app, seen),
        Some(Sink::Message) => {
            return vec![Command::Workspace(workspace::Command::Message(lines(text)))];
        }
        Some(Sink::Agent) => return typed_at_agent(text, app),
        Some(Sink::Terminal) => return typed_at_shell(text, app),
        Some(Sink::Buffer) => return pasted_in_buffer(text, ui, app),
        None => {}
    }
    Vec::new()
}

/// The clipboard into the session's field that has the keys, as typing it would.
fn pasted_in_field(typed: Typed, text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let session = &mut ui.session;
    match typed {
        Typed::Naming => {
            if let Some(naming) = session.naming.as_mut() {
                naming.field.paste(text);
            }
        }
        Typed::Noting => {
            if let Some(noting) = session.noting.as_mut() {
                noting.buffer.edit(&lines(text));
            }
        }
        Typed::Bar(term) => {
            session.bar.of(term).paste(text);
            return keys::searches(ui);
        }
        Typed::Kinds => {
            session.resources.filter.paste(text);
            keys::kinds_changed(ui, app);
        }
        Typed::Search => {
            session.resources.search.paste(text);
            keys::search_changed(ui);
        }
    }
    Vec::new()
}

/// Text for a buffer of many lines: as pasted, its carriage returns gone.
fn lines(text: &str) -> groove_types::Edit {
    groove_types::Edit::Insert(text.replace('\r', ""))
}

fn pasted_in_find(text: &str, ui: &mut Ui, app: &AppState, seen: Seen) -> Vec<Command> {
    if let Some(find) = ui.session.find.as_mut() {
        find.query.paste(text);
    }
    keys::refound(ui, app, seen)
}

/// The clipboard into the buffer the keyboard edits, else the open file.
fn pasted_in_buffer(text: &str, ui: &Ui, app: &AppState) -> Vec<Command> {
    let file = ui.session.tab != crate::views::session::Tab::Resources;
    let editing = crate::editor::Editing::keyed(app, ui);
    let editing = editing.or(file.then_some(crate::editor::Editing::File));
    editing
        .and_then(|one| one.pasted(text))
        .into_iter()
        .collect()
}

/// The clipboard in the search while typing, else at a running sign-in.
fn pasted_in_settings(text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if ui.settings.typing {
        ui.settings.search.paste(text);
        return Vec::new();
    }
    if let Some(field) = ui.settings.draft.as_mut().and_then(|one| one.focused()) {
        field.paste(text.trim());
        return Vec::new();
    }
    if app.config.login.is_none() {
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
