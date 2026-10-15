//! The icon each palette entry draws before its label.

use groove_controllers::{AppState, Command, delivery, session, task, workspace};
use groove_types::DiffView;
use groove_ui_kit::base::mark::Mark;

use super::{Act, Action, Entry, Run};
use crate::keymap::Action as Key;

/// The entry's icon; none for a command no row offers yet.
pub fn mark_of(entry: &Entry, app: &AppState) -> Option<Mark> {
    match &entry.run {
        Run::Settings => Some(Mark::Settings),
        Run::Scope => Some(Mark::Cluster),
        Run::Flow(action) => Some(flowed(*action)),
        Run::Act(act) => acted(act),
        Run::Command(command) => commanded(command, app),
    }
}

fn flowed(action: Action) -> Mark {
    match action {
        Action::AddRepo | Action::AddWorktree => Mark::Add,
        Action::RemoveRepo
        | Action::CloseWorktree
        | Action::ForceCloseWorktree
        | Action::ForceDelete => Mark::Close,
        Action::SelectWorktree | Action::PickBranch => Mark::Git,
        Action::PickRepo => Mark::Repo,
        Action::RenameExplorer => Mark::Explorer,
        Action::AttachCluster | Action::DetachCluster => Mark::Cluster,
    }
}

fn acted(act: &Act) -> Option<Mark> {
    let mark = match act {
        Act::Key(key) => keyed(*key)?,
        Act::View(DiffView::Inline) => Mark::Inline,
        Act::View(DiffView::Split) => Mark::Split,
        Act::Note | Act::Notes(_) => Mark::Note,
        Act::Examine(_) => Mark::Agent,
        Act::Open(_) => Mark::File,
        Act::Discard => Mark::Close,
    };
    Some(mark)
}

fn keyed(key: Key) -> Option<Mark> {
    let mark = match key {
        Key::Board => Mark::Board,
        Key::Overview => Mark::Overview,
        Key::Diff => Mark::Review,
        Key::Files => Mark::Folder,
        Key::Resources => Mark::Cluster,
        Key::FoldSidebar | Key::FocusSidebar | Key::FocusRail => Mark::Sidebar,
        Key::FocusAgent => Mark::Agent,
        Key::FocusCommit => Mark::Commit,
        Key::FocusWorkspace => Mark::File,
        Key::Terminals | Key::NewTerminal => Mark::Terminal,
        Key::Reload => Mark::Reload,
        _ => return None,
    };
    Some(mark)
}

fn commanded(command: &Command, app: &AppState) -> Option<Mark> {
    let mark = match command {
        Command::Session(session::Command::OpenExplorer { .. }) => Mark::Explorer,
        Command::Session(session::Command::Close { .. } | session::Command::Delete { .. }) => {
            Mark::Close
        }
        Command::Session(session::Command::Select { session }) => {
            let open = app
                .session
                .open
                .iter()
                .find(|one| one.session.id == *session)?;
            Mark::of_kind(&open.session.kind)
        }
        Command::Workspace(workspace::Command::Commit) => Mark::Commit,
        Command::Workspace(workspace::Command::Push) => Mark::Ahead,
        Command::Workspace(workspace::Command::Pull) => Mark::Behind,
        Command::Delivery(delivery::Command::BrowseMr { .. }) => Mark::Outward,
        Command::Delivery(delivery::Command::CreateMr) => Mark::Review,
        Command::Delivery(delivery::Command::RefreshMr) => Mark::Reload,
        Command::Delivery(delivery::Command::UpdateMr) => Mark::Ahead,
        Command::Delivery(delivery::Command::CloseMr) => Mark::Close,
        Command::Delivery(delivery::Command::Say(_)) => Mark::Note,
        Command::Agent(_) => Mark::Agent,
        Command::Task(task::Command::Finish { .. }) => Mark::Task,
        Command::Task(task::Command::Browse { .. }) => Mark::Outward,
        Command::Config(_) => Mark::Theme,
        _ => return None,
    };
    Some(mark)
}
