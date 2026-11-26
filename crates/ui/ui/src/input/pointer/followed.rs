//! Ctrl and a click on the conversation: the link or the file its words name, opened.

use groove_controllers::{AppState, Command, session, task, workspace};
use groove_types::{Caret, Selection};

use crate::hit::{Hits, read};
use crate::views::session::Tab;
use crate::{Focus, Ui};

/// The token under the point followed: a URL in the browser, a path in the Files tab at its line.
pub(super) fn followed(
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    point: (f32, f32),
) -> Vec<Command> {
    let Some(token) = read::token(hits.read(), point) else {
        return Vec::new();
    };
    if token.starts_with("http://") || token.starts_with("https://") {
        return vec![Command::Task(task::Command::Browse { url: token })];
    }
    let (path, line) = match token.split_once(':') {
        Some((path, rest)) => (
            path,
            rest.split(':').next().and_then(|one| one.parse().ok()),
        ),
        None => (token.as_str(), None),
    };
    filed(ui, app, path, line)
}

/// A path in the Files tab, its worktree selected first when another holds it.
fn filed(ui: &mut Ui, app: &AppState, path: &str, line: Option<usize>) -> Vec<Command> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let inside = match crate::views::session::agent_pane::within(open, path) {
        Some((worktree, inside)) => {
            if open.state.selected_worktree.as_ref() != Some(&worktree.id) {
                let select = session::Command::SelectWorktree {
                    session: open.session.id.clone(),
                    worktree: worktree.id.clone(),
                };
                out.push(Command::Session(select));
            }
            inside
        }
        None if path.starts_with('/') => return Vec::new(),
        None => path.to_string(),
    };
    (ui.session.tab, ui.session.stowed, ui.focus) = (Tab::Files, false, Focus::Workspace);
    let at = line.map(|line| Selection::at(Caret::new(line.saturating_sub(1), 0)));
    let opened = workspace::Command::OpenFile { path: inside, at };
    out.push(Command::Workspace(opened));
    out
}
