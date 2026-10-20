//! What the language servers found, as the columns a row draws.

use groove_controllers::AppState;
use groove_controllers::workspace_service::display_of;
use groove_types::{Diagnostic, Severity};

pub(super) type Fault = (usize, usize, Severity);

/// What the servers last said of `line` of `path` in the selected worktree.
pub(crate) fn of<'a>(app: &'a AppState, path: &str, line: usize) -> &'a [Diagnostic] {
    let Some(worktree) = app.workspace.worktree.as_ref() else {
        return &[];
    };
    app.workspace.servers.diagnostics_on(worktree, path, line)
}

/// What they cover of `line`; one that covers nothing marks the character it stands on.
pub(super) fn on(found: &[Diagnostic], line: usize, text: &str, width: usize) -> Vec<Fault> {
    let chars = text.chars().count();
    found
        .iter()
        .filter(|one| one.start.line <= line && line <= one.end.line)
        .map(|one| {
            let from = if one.start.line == line {
                one.start.column
            } else {
                0
            };
            let to = if one.end.line == line {
                one.end.column
            } else {
                chars
            };
            let (from, to) = match (from.min(chars), to.min(chars)) {
                (from, to) if from < to => (from, to),
                _ if chars == 0 => (0, 0),
                (from, _) => (from.min(chars - 1), from.min(chars - 1) + 1),
            };
            let column = |at| display_of(text, at, width);
            (column(from), column(to), one.severity)
        })
        .collect()
}

/// How many errors and warnings the servers found in `path`; `None` when none.
pub(crate) fn tally(app: &AppState, path: &str) -> Option<(u32, u32)> {
    let worktree = app.workspace.worktree.as_ref()?;
    let tally = app.workspace.servers.tally(worktree, path);
    (tally != (0, 0)).then_some(tally)
}
