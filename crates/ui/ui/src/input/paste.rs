//! The clipboard into whatever holds the keyboard.

use groove_controllers::{AppState, Command, agent, workspace};

use super::keys;
use super::typed::{self, Typed};
use crate::hit::Hits;
use crate::{Focus, Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;

type Seen<'a> = (&'a Hits, Metrics);

/// The clipboard into whatever holds the keyboard: a field here, a buffer by command.
pub(super) fn pasted(text: &str, ui: &mut Ui, app: &AppState, seen: Seen) -> Vec<Command> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    if let Some(palette) = ui.palette_mut() {
        palette.query.paste(text);
        palette.selected = 0;
        return Vec::new();
    }
    if ui.settings.open {
        return pasted_in_settings(text, ui, app);
    }
    match ui.showing(app) {
        Surface::Board => {
            if ui.board.typing {
                ui.board.filter.paste(text);
                ui.board.offer = 0;
            }
            Vec::new()
        }
        Surface::Session => pasted_in_session(text, ui, app, seen),
    }
}

/// The clipboard into the session's field that has the keys, as typing it would; else the pane.
fn pasted_in_session(text: &str, ui: &mut Ui, app: &AppState, seen: Seen) -> Vec<Command> {
    let session = &mut ui.session;
    match typed::keyed(session) {
        Some(Typed::Naming) => {
            if let Some(naming) = session.naming.as_mut() {
                naming.field.paste(text);
            }
        }
        Some(Typed::Noting) => {
            if let Some(noting) = session.noting.as_mut() {
                let text = text.trim().replace('\r', "");
                noting.buffer.edit(&groove_types::Edit::Insert(text));
            }
        }
        Some(Typed::Bar(term)) => {
            session.bar.of(term).paste(text);
            return keys::searches(ui);
        }
        Some(Typed::Kinds) => {
            session.resources.filter.paste(text);
            keys::kinds_changed(ui, app);
        }
        Some(Typed::Search) => {
            session.resources.search.paste(text);
            keys::search_changed(ui);
        }
        None => return pasted_in_pane(text, ui, app, seen),
    }
    Vec::new()
}

/// The clipboard into the find bar, the commit message, or the pane that has the keys.
fn pasted_in_pane(text: &str, ui: &mut Ui, app: &AppState, seen: Seen) -> Vec<Command> {
    if let Some(find) = ui.session.find.as_mut().filter(|find| find.typing) {
        find.query.paste(text);
        return keys::refound(ui, app, seen);
    }
    if ui.session.composing && ui.focus == Focus::Sidebar {
        let edit = groove_types::Edit::Insert(text.to_string());
        return vec![Command::Workspace(workspace::Command::Message(edit))];
    }
    match ui.focus {
        Focus::Agent => typed_at_agent(text, app),
        Focus::Terminal => typed_at_shell(text, app),
        _ => {
            let file = ui.session.tab != crate::views::session::Tab::Resources;
            let editing = crate::editor::Editing::keyed(app, ui);
            let editing = editing.or(file.then_some(crate::editor::Editing::File));
            editing.and_then(|one| one.paste()).into_iter().collect()
        }
    }
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
