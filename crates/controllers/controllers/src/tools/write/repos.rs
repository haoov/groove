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
    let repo = match repo_of(state, &write) {
        Ok(repo) => repo,
        Err(why) => return write.reply.failed(why),
    };
    let (spec, session, asker) = (spec(&write), write.session, Asker::Agent(write.reply));
    crate::session::add_worktree(state, services, spawner, &session, &repo, spec, asker);
}

const NO_REPO: &str = "name a repo the session already has, or add it with add_task_repo";
const AMBIGUOUS: &str = "more than one repo of the session fits; name it by its slug";

/// The branch to cut and the branch it merges into, as the agent named them.
fn spec(write: &Write) -> WorktreeSpec {
    WorktreeSpec {
        branch: write.text("branch").map(str::to_string),
        target: write.text("target_branch").map(str::to_string),
        track_remote: None,
        mr_head: None,
    }
}

/// The repo the write names among the session's own, or its only one; a name two hold is ambiguous.
fn repo_of(state: &AppState, write: &Write) -> Result<groove_types::RepoId, &'static str> {
    let open = state.session.get(&write.session).ok_or(NO_REPO)?;
    let held: Vec<&groove_types::Repo> = match write.text("repo") {
        Some(named) => open
            .repos
            .iter()
            .filter(|repo| repo.slug() == named || repo.project == named)
            .collect(),
        None => open.repos.iter().collect(),
    };
    match held.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => Err(NO_REPO),
        _ => Err(AMBIGUOUS),
    }
}
