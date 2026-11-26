//! An edit's file opened in the Diff tab, scrolled to its hunk once the diff holds it.

use groove_controllers::{AppState, Command, session};

use crate::Ui;
use crate::views::session::Tab;
use groove_ui_kit::base::ctx::Metrics;

/// The rows kept above the hunk's first line.
const ABOVE: usize = 3;

/// The Diff tab on the worktree that holds `path`, the hunk at `line` waited for.
pub(super) fn opened(ui: &mut Ui, app: &AppState, path: &str, line: u32) -> Vec<Command> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let Some((worktree, inside)) = crate::views::session::agent_pane::within(open, path) else {
        return Vec::new();
    };
    (ui.session.tab, ui.session.going) = (Tab::Diff, None);
    ui.session.stowed = false;
    ui.focus = crate::Focus::Workspace;
    ui.session.revealing = Some((inside, line));
    if open.state.selected_worktree.as_ref() == Some(&worktree.id) {
        return Vec::new();
    }
    let select = session::Command::SelectWorktree {
        session: open.session.id.clone(),
        worktree: worktree.id.clone(),
    };
    vec![Command::Session(select)]
}

/// The stream scrolled to the hunk waited for, once the diff holds its file; another tab lets it go.
pub fn reveal(ui: &mut Ui, app: &AppState, metrics: Metrics) {
    let Some((path, line)) = ui.session.revealing.as_ref() else {
        return;
    };
    if ui.session.tab != Tab::Diff {
        ui.session.revealing = None;
        return;
    }
    let changes = &app.workspace.changes;
    let Some(row) = changes
        .row_of(path, *line)
        .or_else(|| changes.head_of(path))
    else {
        return;
    };
    ui.session.diff = row.saturating_sub(ABOVE) as f32 * metrics.tokens().code_line;
    ui.session.revealing = None;
}
