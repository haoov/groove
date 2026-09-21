//! The branch's history: the commits it holds, and one of them shown as a change.

use groove_workspace_service::{COMMITS_MAX, at_commit, commits};

use super::worktree_dir;
use crate::{AppState, Continuation, Services, Spawner};

/// The newest commits of the selected worktree's branch.
pub(super) fn list(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(dir) = worktree_dir(state) else {
        state.workspace.logged = None;
        return state.workspace.log.clear();
    };
    state.workspace.logged = super::selected(state);
    let base = base_of(state);
    let job = state.begin("reading the commits");
    spawner.spawn(Box::pin(async move {
        let read = commits(&dir, base.as_deref(), COMMITS_MAX).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match read {
                Err(e) => state.errors.push(e),
                Ok(log) => state.workspace.log = log,
            }
        }) as Continuation
    }));
}

/// The branch the work is based on, which the commits above it are the branch's own.
fn base_of(state: &AppState) -> Option<String> {
    let open = state.session.selected()?;
    let worktree = open.selected_worktree()?;
    Some(worktree.base_ref.clone().unwrap_or("origin/HEAD".into()))
}

/// One commit shown as the change it made, which nothing may write.
pub(super) fn open(state: &mut AppState, spawner: &dyn Spawner, sha: String) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let Some(entry) = state
        .workspace
        .log
        .iter()
        .find(|one| one.sha == sha)
        .cloned()
    else {
        return;
    };
    let job = state.begin(format!("reading {}", entry.short_sha));
    spawner.spawn(Box::pin(async move {
        let read = at_commit(&dir, &sha).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match read {
                Err(e) => state.errors.push(e),
                Ok((files, changes)) => shown(state, entry, files, changes),
            }
        }) as Continuation
    }));
}

/// What the commit changed, in place of the working tree.
fn shown(
    state: &mut AppState,
    entry: groove_types::CommitEntry,
    files: Vec<groove_types::FileDiff>,
    changes: groove_workspace_service::Changes,
) {
    let Some(worktree) = super::selected(state) else {
        return;
    };
    state.workspace.opened = None;
    state.workspace.loaded(worktree, files, changes);
    state.workspace.commit = Some(entry);
}

/// The working tree again, in place of the commit.
pub(super) fn leave(state: &mut AppState, spawner: &dyn Spawner) {
    if state.workspace.commit.take().is_none() {
        return;
    }
    state.workspace.opened = None;
    super::diff::reread(state, spawner);
}
