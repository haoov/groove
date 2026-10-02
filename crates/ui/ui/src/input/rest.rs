//! The caret resting on a line: when it came to rest, and the blame it then asks for.

use groove_controllers::{AppState, Command, workspace};
use groove_ui_kit::base::tokens::REST_MS;

use crate::Ui;
use crate::views::session::diff::{rested, spot};

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

/// How many milliseconds the caret has yet to rest, while it is resting.
pub fn resting(ui: &Ui, tick: u64) -> Option<u64> {
    let (_, since) = ui.session.rest.as_ref()?;
    (since + REST_MS).checked_sub(tick).filter(|left| *left > 0)
}
