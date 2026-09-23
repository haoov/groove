//! What the agent attaches to its own session: a repo, or another branch of one.

use groove_types::WorktreeSpec;

use super::Write;
use crate::asker::Asker;
use crate::{AppState, Services, Spawner};

/// A repo of the pool attached to the session, with a worktree cut for it.
pub(super) fn add_repo(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(name) = write.text("repo").map(str::to_string) else {
        return write.reply.failed("add_task_repo needs a repo");
    };
    let (spec, session, asker) = (spec(&write), write.session, Asker::Agent(write.reply));
    crate::session::add_repo(state, services, spawner, &session, &name, spec, asker);
}

/// Another branch of a repo the session already has, as a second worktree.
pub(super) fn add_worktree(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    if write.text("branch").is_none() {
        return write.reply.failed("add_task_worktree needs a branch");
    }
    let Some(repo) = repo_of(state, &write) else {
        return write.reply.failed(NO_REPO);
    };
    let (spec, session, asker) = (spec(&write), write.session, Asker::Agent(write.reply));
    crate::session::add_worktree(state, services, spawner, &session, &repo, spec, asker);
}

const NO_REPO: &str = "name a repo the session already has, or add it with add_task_repo";

/// The branch to cut and the branch it merges into, as the agent named them.
fn spec(write: &Write) -> WorktreeSpec {
    WorktreeSpec {
        branch: write.text("branch").map(str::to_string),
        target: write.text("target_branch").map(str::to_string),
        track_remote: None,
    }
}

/// The repo the write names among the session's own, or its only one.
fn repo_of(state: &AppState, write: &Write) -> Option<groove_types::RepoId> {
    let open = state.session.get(&write.session)?;
    let Some(named) = write.text("repo") else {
        return match open.repos.as_slice() {
            [one] => Some(one.id.clone()),
            _ => None,
        };
    };
    open.repos
        .iter()
        .find(|repo| repo.slug() == named || repo.project == named)
        .map(|repo| repo.id.clone())
}
