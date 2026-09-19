//! What the palette lists: one group per capability, in the order it shows them.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, session};
use groove_types::SessionKind;

use super::{Action, Entry};

pub fn entries(app: &AppState) -> Vec<Entry> {
    let mut out = vec![Entry::command(
        "Session",
        "New explorer",
        Command::Session(session::Command::OpenExplorer { title: None }),
    )];
    let Some(open) = app.session.selected() else {
        return out;
    };
    out.extend(repos(open));
    out.extend(worktrees(open));
    out.extend(explorer(open));
    out.extend(sessions(app, open));
    out
}

/// What can be done to the session's repos.
fn repos(open: &Open) -> Vec<Entry> {
    let mut out = vec![Entry::flow("Session", "Add repo", Action::AddRepo)];
    if !open.repos.is_empty() {
        out.push(Entry::flow("Session", "Add worktree", Action::AddWorktree));
        out.push(Entry::flow("Session", "Remove repo", Action::RemoveRepo));
    }
    out
}

fn worktrees(open: &Open) -> Vec<Entry> {
    let mut out = Vec::new();
    if !open.worktrees.is_empty() {
        out.push(Entry::flow(
            "Session",
            "Close worktree",
            Action::CloseWorktree,
        ));
    }
    if open.worktrees.len() > 1 {
        out.push(Entry::flow(
            "Session",
            "Select worktree",
            Action::SelectWorktree,
        ));
    }
    out
}

/// What only an explorer offers: its name, and throwing it away.
fn explorer(open: &Open) -> Vec<Entry> {
    if !matches!(open.session.kind, SessionKind::Explorer) {
        return Vec::new();
    }
    vec![
        Entry::flow("Session", "Rename explorer", Action::RenameExplorer),
        Entry::command(
            "Session",
            "Delete session",
            Command::Session(session::Command::Delete {
                session: open.session.id.clone(),
            }),
        ),
    ]
}

/// Closing this session, and switching to any other open one.
fn sessions(app: &AppState, open: &Open) -> Vec<Entry> {
    let id = open.session.id.clone();
    let mut out = vec![Entry::command(
        "Session",
        "Close session",
        Command::Session(session::Command::Close {
            session: id.clone(),
        }),
    )];
    for other in app.session.open.iter().filter(|o| o.session.id != id) {
        out.push(Entry::command(
            "Session",
            format!("Switch to {}", other.session.title),
            Command::Session(session::Command::Select {
                session: other.session.id.clone(),
            }),
        ));
    }
    out
}
