//! What a thread's own buttons do: a reply started, or the forge asked.

use groove_controllers::delivery::{self, ThreadAct};
use groove_controllers::{AppState, Command};
use groove_types::Note;

use crate::Ui;
use crate::hit::NoteButton;
use crate::views::session::Noting;

/// What a thread's own button does: fold it, start a reply, or resolve it now.
pub(crate) fn noted(
    ui: &mut Ui,
    app: &AppState,
    thread: String,
    button: NoteButton,
) -> Vec<Command> {
    match button {
        NoteButton::Fold => {
            let unfolded = &mut ui.session.unfolded;
            match unfolded.iter().position(|one| *one == thread) {
                Some(at) => drop(unfolded.remove(at)),
                None => unfolded.push(thread),
            }
            Vec::new()
        }
        NoteButton::Reply => {
            replying(ui, app, thread);
            Vec::new()
        }
        NoteButton::Resolve => {
            let resolve = !held(app, &thread).is_some_and(|note| note.resolved);
            let act = ThreadAct::Resolve { thread, resolve };
            vec![Command::Delivery(delivery::Command::Thread(act))]
        }
    }
}

/// An empty row under the thread, for the words that answer it.
fn replying(ui: &mut Ui, app: &AppState, thread: String) {
    let Some(anchor) = held(app, &thread).and_then(|note| note.anchor.clone()) else {
        return;
    };
    ui.session.noting = Some(Noting::reply(anchor, thread));
    ui.focus = crate::Focus::Workspace;
}

fn held<'a>(app: &'a AppState, thread: &str) -> Option<&'a Note> {
    app.delivery.shown.iter().find(|note| note.thread == thread)
}
