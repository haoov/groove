//! The `session` controller: one function per user action on the `session` service.

mod clusters;
mod delete;
pub(crate) mod feed;
mod open;
mod rail;
mod repos;
mod review;

use groove_types::{Attached, RepoId, SessionId, WorktreeId, WorktreeSpec};

use crate::asker::Asker;

pub use delete::{delete, delete_local, force_delete};
pub(crate) use open::{Start, begun, begun_with};
pub use open::{close, open, open_explorer, restore, select};
pub use rail::{list, refresh_status, rename_explorer};
pub(crate) use rail::{listed, set_auto_approve};
pub(crate) use repos::added;
pub use repos::{
    add_repo, add_worktree, close_worktree, list_branches, list_repos, remove_repo, select_worktree,
};
pub use review::open_review;

use crate::{AppState, Services, Spawner};

/// The queue's own row for this MR, which the command names by project and number.
fn reviewed(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    project: &str,
    iid: u64,
) {
    let found = state
        .delivery
        .reviews
        .iter()
        .find(|mr| mr.project == project && mr.iid == iid)
        .cloned();
    if let Some(at) = found {
        open_review(state, services, spawner, &at);
    }
}

/// The grid a terminal starts on; its pane resizes it on its first frame.
pub const FIRST_SIZE: (u16, u16) = (80, 24);

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `session.restore`: the rail as it was when the app last closed, agents started.
    Restore,
    /// `session.open_explorer`: a session with no ticket yet, its agent started.
    OpenExplorer { title: Option<String> },
    /// `session.open_review`: the session that reviews one of the queue's MRs.
    OpenReview { project: String, iid: u64 },
    /// `session.rename_explorer`
    RenameExplorer { session: SessionId, title: String },
    /// `session.delete`: the agent ended, the worktrees and branches removed with any work in them.
    Delete { session: SessionId },
    /// `session.delete_local`: the session and its worktrees gone from this machine.
    DeleteLocal { session: SessionId },
    /// `session.force_delete`: any session gone from this machine, changes and all.
    ForceDelete { session: SessionId },
    /// `session.select`: make it the current one.
    Select { session: SessionId },
    /// `session.open`: a session picked anywhere, back on the rail if it had left.
    Open { session: SessionId },
    /// `session.list`: every session that lives on disk.
    List,
    /// `session.close`: end the agent, drop the row; the session stays on disk.
    Close { session: SessionId },
    /// `session.add_repo`: a pool repo by name, or a URL to clone, its first worktree cut.
    AddRepo {
        session: SessionId,
        name: String,
        spec: WorktreeSpec,
    },
    /// `session.remove_repo`: close its worktrees, detach it.
    RemoveRepo {
        session: SessionId,
        repo: RepoId,
        force: bool,
    },
    /// `session.add_worktree`: another branch of a repo the session holds.
    AddWorktree {
        session: SessionId,
        repo: RepoId,
        spec: WorktreeSpec,
    },
    /// `session.select_worktree`: the one the workspace follows.
    SelectWorktree {
        session: SessionId,
        worktree: WorktreeId,
    },
    /// `session.close_worktree`: the directory and the local branch go; origin keeps its copy.
    CloseWorktree {
        session: SessionId,
        worktree: WorktreeId,
        force: bool,
    },
    /// `session.list_repos`: refresh the pool listing for the pickers.
    ListRepos,
    /// `session.list_branches`: refresh origin's heads of a repo for the pickers.
    ListBranches { repo: RepoId },
    /// `session.attach_cluster`: a context Groove knows, on a namespace or the whole cluster.
    AttachCluster {
        session: SessionId,
        attached: Attached,
    },
    /// `session.detach_cluster`: one context and namespace the session no longer holds.
    DetachCluster {
        session: SessionId,
        attached: Attached,
    },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Restore => "session.restore",
            Command::OpenExplorer { .. } => "session.open_explorer",
            Command::OpenReview { .. } => "session.open_review",
            Command::RenameExplorer { .. } => "session.rename_explorer",
            Command::Delete { .. } => "session.delete",
            Command::DeleteLocal { .. } => "session.delete_local",
            Command::ForceDelete { .. } => "session.force_delete",
            Command::Select { .. } => "session.select",
            Command::Open { .. } => "session.open",
            Command::List => "session.list",
            Command::Close { .. } => "session.close",
            Command::AddRepo { .. } => "session.add_repo",
            Command::RemoveRepo { .. } => "session.remove_repo",
            Command::AddWorktree { .. } => "session.add_worktree",
            Command::SelectWorktree { .. } => "session.select_worktree",
            Command::CloseWorktree { .. } => "session.close_worktree",
            Command::ListRepos => "session.list_repos",
            Command::ListBranches { .. } => "session.list_branches",
            Command::AttachCluster { .. } => "session.attach_cluster",
            Command::DetachCluster { .. } => "session.detach_cluster",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Restore => restore(services, spawner),
        Command::OpenExplorer { title } => {
            open_explorer(state, services, spawner, title.as_deref())
        }
        Command::OpenReview { project, iid } => reviewed(state, services, spawner, &project, iid),
        Command::RenameExplorer { session, title } => {
            rename_explorer(state, services, spawner, &session, &title)
        }
        Command::Delete { session } => delete(state, services, spawner, &session, true),
        Command::DeleteLocal { session } => delete_local(state, services, spawner, &session),
        Command::ForceDelete { session } => force_delete(state, services, spawner, &session),
        Command::Select { session } => select(state, services, spawner, &session),
        Command::Open { session } => open(state, services, spawner, &session),
        Command::List => list(services, spawner),
        Command::Close { session } => close(state, services, spawner, &session),
        Command::AddRepo {
            session,
            name,
            spec,
        } => add_repo(state, services, spawner, &session, &name, spec, Asker::Ui),
        Command::RemoveRepo {
            session,
            repo,
            force,
        } => remove_repo(state, services, spawner, &session, &repo, force),
        Command::AddWorktree {
            session,
            repo,
            spec,
        } => add_worktree(state, services, spawner, &session, &repo, spec, Asker::Ui),
        Command::SelectWorktree { session, worktree } => {
            select_worktree(state, services, spawner, &session, &worktree)
        }
        Command::CloseWorktree {
            session,
            worktree,
            force,
        } => close_worktree(state, services, spawner, &session, &worktree, force),
        Command::ListRepos => list_repos(services, spawner),
        Command::ListBranches { repo } => list_branches(services, spawner, &repo),
        cluster => clusters::dispatch(cluster, state, services, spawner),
    }
}
