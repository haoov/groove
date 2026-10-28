//! What the palette lists: one group per capability, in the order it shows them.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, session};
use groove_types::SessionKind;

use super::{Action, Entry};

pub fn entries(app: &AppState) -> Vec<Entry> {
    let mut out = vec![
        Entry::command(
            "Session",
            "New explorer",
            Command::Session(session::Command::OpenExplorer { title: None }),
        ),
        Entry {
            group: "Config",
            label: "Settings".into(),
            run: super::Run::Settings,
        },
    ];
    if !app.session.living.is_empty() {
        out.push(Entry::flow(
            "Session",
            "Force delete session",
            Action::ForceDelete,
        ));
    }
    let Some(open) = app.session.selected() else {
        out.extend(super::actions::actions(app));
        return out;
    };
    out.extend(repos(open));
    out.extend(worktrees(open));
    out.extend(clusters(app, open));
    out.extend(explorer(open));
    out.extend(sessions(app, open));
    out.extend(super::actions::actions(app));
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

/// Attach while Groove knows a context; detach while the session holds one.
fn clusters(app: &AppState, open: &Open) -> Vec<Entry> {
    let mut out = Vec::new();
    let keyed = |label: &str, action| Entry {
        group: "Session",
        label: label.into(),
        run: super::Run::Act(super::Act::Key(action)),
    };
    if !app.config.clusters().is_empty() {
        out.push(Entry::flow(
            "Session",
            "Attach cluster",
            Action::AttachCluster,
        ));
        out.push(keyed("Pick contexts", crate::keymap::Action::SelectContext));
    }
    if open.holds_clusters() {
        out.push(Entry::flow(
            "Session",
            "Detach cluster",
            Action::DetachCluster,
        ));
        out.push(keyed(
            "Pick namespaces",
            crate::keymap::Action::SelectNamespace,
        ));
        out.push(Entry::flow("Session", "Pick a kind", Action::PickKind));
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
        out.push(Entry::flow(
            "Session",
            "Force close worktree",
            Action::ForceCloseWorktree,
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

/// Closing this session, and switching to another open one.
fn sessions(app: &AppState, open: &Open) -> Vec<Entry> {
    let id = open.session.id.clone();
    let mut out = vec![Entry::command(
        "Session",
        "Close session",
        Command::Session(session::Command::Close {
            session: id.clone(),
        }),
    )];
    if app.session.open.iter().any(|one| one.session.id != id) {
        out.push(Entry::flow(
            "Session",
            "Switch session",
            Action::SelectSession,
        ));
    }
    out
}
