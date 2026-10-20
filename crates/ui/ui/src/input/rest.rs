//! When the caret and the pointer came to rest on the open file, and what each then asks for.

use groove_controllers::{AppState, Command, workspace};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::{REST_MS, WORD_MS};

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

/// How many milliseconds the caret or the pointer has yet to rest, the soonest of them.
pub fn resting(ui: &Ui, tick: u64) -> Option<u64> {
    let caret = ui.session.rest.as_ref().map(|(_, since)| since + REST_MS);
    let pointer = ui
        .session
        .pointer_rest
        .as_ref()
        .map(|(_, since)| since + REST_MS);
    let unread = ui.session.word.is_none();
    let word = ui.session.word_rest.as_ref().filter(|_| unread);
    let left = |due: u64| due.checked_sub(tick).filter(|left| *left > 0);
    [caret, pointer, word.map(|(_, since)| since + WORD_MS)]
        .into_iter()
        .flatten()
        .filter_map(left)
        .min()
}

/// The word the caret stands in, read once the caret and the text have rested.
pub fn word(ui: &mut Ui, app: &AppState, tick: u64) {
    let keyed = ui.focus == crate::Focus::Workspace && !ui.session.typing();
    let here = keyed.then_some(()).and_then(|_| {
        let editor = crate::views::session::diff::edited(app, ui)?;
        let at = editor.buffer.caret();
        let spot = Pointed {
            path: editor.path.to_string(),
            at,
        };
        Some((spot, editor.buffer.revision()))
    });
    if ui.session.word_rest.as_ref().map(|(at, _)| at) != here.as_ref() {
        ui.session.word_rest = here.map(|at| (at, tick));
        ui.session.word = None;
        return;
    }
    let Some((at, since)) = ui.session.word_rest.as_ref() else {
        return;
    };
    if tick >= since + WORD_MS && ui.session.word.is_none() {
        ui.session.word = word_at(app, ui, at.0.at);
    }
}

/// The run of word characters around `at` in the edited buffer.
fn word_at(app: &AppState, ui: &Ui, at: groove_types::Caret) -> Option<String> {
    let editor = crate::views::session::diff::edited(app, ui)?;
    let line = editor.buffer.line(at.line)?;
    groove_controllers::workspace_service::word_around(&line, at.column)
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
