//! The caret resting on a line, and the pointer on a spot of the open file: when each came to
//! rest, and what it then asks for.

use groove_controllers::{AppState, Command, workspace};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::REST_MS;

use crate::Ui;
use crate::hit::Hits;
use crate::views::session::diff::{Pointed, rested, spot};

/// The blame of the caret's file once it has rested long enough and none is kept.
pub fn rest(ui: &mut Ui, app: &AppState, tick: u64) -> Option<Command> {
    let here = spot(ui, app);
    if ui.session.rest.as_ref().map(|(at, _)| at) != here.as_ref() {
        ui.session.rest = here.map(|at| (at, tick));
        return None;
    }
    let at = rested(ui, app, tick)?;
    let worktree = app.session.selected_worktree()?;
    let read = app.workspace.read_of(&at.path);
    let owed = app.workspace.blames.owed(&worktree.id, &at.path, read);
    owed.then_some(Command::Workspace(workspace::Command::Blame {
        path: at.path,
    }))
}

/// How many milliseconds the caret or the pointer has yet to rest, the sooner of the two.
pub fn resting(ui: &Ui, tick: u64) -> Option<u64> {
    let caret = ui.session.rest.as_ref().map(|(_, since)| *since);
    let pointer = ui.session.pointer_rest.as_ref().map(|(_, since)| *since);
    let left = |since: u64| (since + REST_MS).checked_sub(tick).filter(|left| *left > 0);
    caret
        .and_then(left)
        .into_iter()
        .chain(pointer.and_then(left))
        .min()
}

/// Where the pointer stands in an open file. True when it moved to another spot.
pub fn pointed(ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) -> bool {
    let here = super::pointer::at(ui, app, hits, metrics, ui.at)
        .filter(|(path, _)| app.workspace.buffer(path).is_some())
        .map(|(path, at)| Pointed { path, at });
    let moved = ui.session.pointed != here;
    ui.session.pointed = here;
    moved
}

/// What the servers say of the spot the pointer has rested on, asked once.
pub fn hovering(ui: &mut Ui, app: &AppState, tick: u64) -> Option<Command> {
    let here = ui.session.pointed.clone();
    if ui.session.pointer_rest.as_ref().map(|(at, _)| at) != here.as_ref() {
        ui.session.pointer_rest = here.map(|at| (at, tick));
        return None;
    }
    let (at, since) = ui.session.pointer_rest.as_ref()?;
    if tick < since + REST_MS {
        return None;
    }
    let hovered = app.workspace.servers.hovered.as_ref();
    let asked = hovered.is_some_and(|one| !one.caret && one.path == at.path && one.at == at.at);
    (!asked).then(|| {
        Command::Workspace(workspace::Command::Hover {
            path: at.path.clone(),
            at: at.at,
            caret: false,
        })
    })
}
