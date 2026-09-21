//! What a click on the sidebar's own rows does: its scope, its twisties, its bars.

use groove_controllers::Command;

use crate::Ui;
use crate::views::session::{Scope, Term};

/// Which files the sidebar lists; the frame asks for the walk a tree needs.
pub(super) fn scoped(ui: &mut Ui, scope: Scope) -> Vec<Command> {
    ui.session.scope = scope;
    ui.session.files = 0.0;
    Vec::new()
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
