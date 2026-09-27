//! What a note's own buttons do: its words written again, or the forge asked.

use groove_controllers::delivery::{self, NoteAct, ThreadAct};
use groove_controllers::{AppState, Command};
use groove_types::{AnnotationId, Note, NoteOrigin};

use crate::Ui;
use crate::base::hit::NoteButton;
use crate::views::session::Noting;

/// What a note's own button does: rewrite it in place, or act on it now.
pub(crate) fn noted(
    ui: &mut Ui,
    app: &AppState,
    origin: NoteOrigin,
    button: NoteButton,
) -> Vec<Command> {
    let act = match origin {
        NoteOrigin::Local(id) => return own(ui, app, id, button),
        NoteOrigin::Thread(thread) => thread_act(ui, app, thread, button),
    };
    act.map(Command::Delivery).into_iter().collect()
}

/// What a button does to one of this session's own notes.
fn own(ui: &mut Ui, app: &AppState, id: AnnotationId, button: NoteButton) -> Vec<Command> {
    let act = match button {
        NoteButton::Edit => return editing(ui, app, id),
        NoteButton::Resolve => Some(delivery::Command::Note(resolving(app, id))),
        NoteButton::Delete => Some(delivery::Command::Note(NoteAct::Delete { id })),
        NoteButton::Post => Some(delivery::Command::Thread(ThreadAct::Post { id })),
        NoteButton::Reply => None,
    };
    act.map(Command::Delivery).into_iter().collect()
}

/// What a button does to a thread the forge holds.
fn thread_act(
    ui: &mut Ui,
    app: &AppState,
    thread: String,
    button: NoteButton,
) -> Option<delivery::Command> {
    match button {
        NoteButton::Reply => {
            replying(ui, app, thread);
            None
        }
        NoteButton::Resolve => Some(delivery::Command::Thread(ThreadAct::Resolve {
            resolve: !resolved(app, &thread),
            thread,
        })),
        _ => None,
    }
}

/// An empty row under the thread, for the words that answer it.
fn replying(ui: &mut Ui, app: &AppState, thread: String) {
    let Some(anchor) = held_thread(app, &thread).and_then(|note| note.anchor.clone()) else {
        return;
    };
    ui.session.noting = Some(Noting::reply(anchor, thread));
}

fn resolved(app: &AppState, thread: &str) -> bool {
    held_thread(app, thread).is_some_and(|note| note.resolved)
}

fn held_thread<'a>(app: &'a AppState, thread: &str) -> Option<&'a Note> {
    app.delivery
        .shown
        .iter()
        .find(|note| matches!(&note.origin, NoteOrigin::Thread(id) if id == thread))
}

/// The note opened in its own row again, with what it says already in it.
fn editing(ui: &mut Ui, app: &AppState, id: AnnotationId) -> Vec<Command> {
    let Some(note) = held(app, &id) else {
        return Vec::new();
    };
    let Some(anchor) = note.anchor.clone() else {
        return Vec::new();
    };
    let said = note
        .opening()
        .map(|said| said.body.clone())
        .unwrap_or_default();
    ui.session.noting = Some(Noting::over(anchor, id, &said));
    Vec::new()
}

/// A note resolved, or opened again when it already is.
fn resolving(app: &AppState, id: AnnotationId) -> NoteAct {
    let resolved = held(app, &id).is_some_and(|note| note.resolved);
    match resolved {
        true => NoteAct::Reopen { id },
        false => NoteAct::Resolve { id },
    }
}

fn held<'a>(app: &'a AppState, id: &AnnotationId) -> Option<&'a Note> {
    app.delivery.shown.iter().find(|note| note.id() == Some(id))
}
