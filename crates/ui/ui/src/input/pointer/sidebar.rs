//! What a click on the sidebar's own rows does: its scope, its twisties, its bars.

use groove_controllers::{AppState, Command};

use crate::Ui;
use crate::views::session::{Pane, Scope, Term};

/// Which files the sidebar lists; the frame asks for the walk a tree needs.
pub(super) fn scoped(ui: &mut Ui, scope: Scope) -> Vec<Command> {
    ui.session.scope = scope;
    ui.session.files = 0.0;
    Vec::new()
}

/// One of the sidebar's three lists up; the frame reads what it shows.
pub(super) fn paned(ui: &mut Ui, pane: Pane) -> Vec<Command> {
    ui.session.pane = pane;
    ui.session.files = 0.0;
    Vec::new()
}

/// One note of the list, which opens the line it stands on.
pub(super) fn note_at(
    ui: &mut Ui,
    app: &AppState,
    metrics: crate::ctx::Metrics,
    at: usize,
) -> Vec<Command> {
    let Some(anchor) = app
        .workspace
        .notes
        .get(at)
        .and_then(|note| note.anchor.clone())
    else {
        return Vec::new();
    };
    let line = anchor.start_line as usize;
    ui.focus = crate::Focus::Workspace;
    ui.session.view = groove_types::DiffView::Editor;
    let above = line.saturating_sub(crate::tokens::ABOVE_MATCH);
    ui.session.diff = above as f32 * metrics.tokens().line;
    let open = groove_controllers::workspace::Command::OpenFile {
        path: anchor.path,
        at: Some(groove_types::Selection::at(groove_types::Caret::new(
            line, 0,
        ))),
    };
    vec![Command::Workspace(open)]
}

/// One directory of the explorer opened, or shut again.
pub(super) fn twisty(ui: &mut Ui, path: String) -> Vec<Command> {
    if !ui.session.opened.remove(&path) {
        ui.session.opened.insert(path);
    }
    Vec::new()
}

/// The keyboard into one term of the sidebar's bar.
pub(super) fn narrowing(ui: &mut Ui, term: Term) -> Vec<Command> {
    ui.session.composing = false;
    ui.session.bar.focus(term);
    Vec::new()
}

/// The keyboard back into the find bar, at the end of what it holds.
pub(super) fn finding(ui: &mut Ui) -> Vec<Command> {
    if let Some(find) = ui.session.find.as_mut() {
        find.typing = true;
        find.query.end();
    }
    Vec::new()
}
