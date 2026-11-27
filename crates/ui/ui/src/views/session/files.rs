//! The Diff and Files sidebars' lists: the change's files, the commits, the threads, the worktree's tree.

mod commits;
pub(crate) mod explorer;
pub(crate) mod moded;
mod notes;
mod rows;
mod stops;
mod tree;
pub(crate) mod walked;

pub(crate) use stops::{Stop, stops};
pub(crate) use tree::reads_as;

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::FileDiff;

use super::state::Pane;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;

/// What a pane's own name counts beside it.
fn counted(app: &AppState, pane: Pane) -> usize {
    match pane {
        Pane::Files => changed(app).len(),
        Pane::Commits => app.workspace.log.iter().filter(|one| !one.is_base).count(),
        Pane::Notes => app
            .delivery
            .shown
            .iter()
            .filter(|one| !one.resolved)
            .count(),
    }
}

/// Whether the list is the whole worktree, which a query flattens back to matches.
pub(crate) fn browsing(ui: &Ui) -> bool {
    ui.session.mode == super::Mode::Files
}

/// Whether the commits list still needs this worktree's commits read.
pub(crate) fn needs_commits(app: &AppState, ui: &Ui) -> bool {
    let selected = app
        .session
        .selected_worktree()
        .map(|worktree| worktree.id.clone());
    ui.session.pane == Pane::Commits && selected.is_some() && app.workspace.logged != selected
}

/// Whether the explorer still needs the worktree walked before it can draw a tree.
pub(crate) fn needs_walk(app: &AppState, ui: &Ui) -> bool {
    browsing(ui) && holds(app) && app.workspace.paths().is_empty() && !app.workspace.walking
}

fn holds(app: &AppState) -> bool {
    app.session.selected_worktree().is_some()
}

/// What the tree has to say instead of rows.
fn empty(app: &AppState) -> &'static str {
    match (holds(app), app.workspace.walking) {
        (false, _) => "no worktree to read",
        (true, true) => "reading the worktree…",
        (true, false) => "nothing in this worktree",
    }
}

/// A question in the row's own place: `yes`, or keep.
pub(crate) fn asking(ctx: &mut Ctx, line: Rect, question: &str, yes: &str) {
    let answers = [(yes, Target::Discard), ("keep", Target::Keep)];
    groove_ui_kit::widgets::question(ctx, line, (question, None), answers);
}

/// The files of the worktree the session points at, never another's.
pub(crate) fn changed(app: &AppState) -> &[FileDiff] {
    let selected = app.session.selected_worktree().map(|worktree| &worktree.id);
    app.workspace.files_of(selected)
}

/// Whether a note of this session stands on a file.
pub(super) fn noted(app: &AppState, path: &str) -> bool {
    app.delivery
        .shown
        .iter()
        .any(|note| note.anchor.as_ref().is_some_and(|one| one.path == path))
}

/// Where a list row's text starts past the mark that leads it at `indent`.
pub(crate) fn text_at(ctx: &Ctx, indent: f32) -> f32 {
    indent + ctx.tokens.icon + ctx.tokens.xs
}
