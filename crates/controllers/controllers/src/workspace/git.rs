//! What git is asked to do: the index, a commit, a push, a pull, a change thrown away.

use groove_workspace_service::Buffer;

use super::diff::{load, reread};
use super::worktree_dir;
use crate::{AppState, Continuation, Services, Spawner};

/// What one action does against the remote or the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Remote {
    Push,
    Pull,
}

impl Remote {
    fn label(self) -> &'static str {
        match self {
            Remote::Push => "pushing",
            Remote::Pull => "pulling",
        }
    }
}

/// One action against the remote. HEAD may move, so everything is read again.
pub(super) fn remote(state: &mut AppState, spawner: &dyn Spawner, act: Remote) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let Some(worktree) = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .cloned()
    else {
        return;
    };
    let job = state.begin(act.label());
    spawner.spawn(Box::pin(async move {
        let done = match act {
            Remote::Push => groove_workspace_service::push(&dir, &worktree.branch).await,
            Remote::Pull => groove_workspace_service::pull(&dir).await,
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                if let Err(e) = done {
                    state.errors.push(e);
                }
                crate::session::refresh_status(state, services, spawner);
                reread(state, spawner);
            },
        ) as Continuation
    }));
}

/// Every change in the worktree, thrown away.
pub(super) fn discard_all(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let paths: Vec<String> = state
        .workspace
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    if paths.is_empty() {
        return;
    }
    let job = state.begin("discarding every change");
    spawner.spawn(Box::pin(async move {
        let done = groove_workspace_service::discard(&dir, &paths).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok(()) => load(state, spawner),
                    Err(e) => state.errors.push(e),
                }
            },
        ) as Continuation
    }));
}

/// What one action does to the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Act {
    Stage,
    Unstage,
    Discard,
}

impl Act {
    fn label(self) -> &'static str {
        match self {
            Act::Stage => "staging",
            Act::Unstage => "unstaging",
            Act::Discard => "discarding",
        }
    }
}

/// One path into the index, out of it, or thrown away.
pub(super) fn index(state: &mut AppState, spawner: &dyn Spawner, act: Act, path: String) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let job = state.begin(format!("{} {path}", act.label()));
    spawner.spawn(Box::pin(async move {
        let paths = [path];
        let done = match act {
            Act::Stage => groove_workspace_service::stage(&dir, &paths).await,
            Act::Unstage => groove_workspace_service::unstage(&dir, &paths).await,
            Act::Discard => groove_workspace_service::discard(&dir, &paths).await,
        };
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok(()) => load(state, spawner),
                    Err(e) => state.errors.push(e),
                }
            },
        ) as Continuation
    }));
}

/// The index committed, then every side of the diff read again.
pub(super) fn commit(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let message = state.workspace.message.text();
    if message.trim().is_empty() {
        return;
    }
    let job = state.begin("committing");
    spawner.spawn(Box::pin(async move {
        let done = groove_workspace_service::commit(&dir, message.trim()).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok(()) => committed(state, spawner),
                    Err(e) => state.errors.push(e),
                }
            },
        ) as Continuation
    }));
}

/// The message is spent, and every side of the diff is read again.
pub(super) fn committed(state: &mut AppState, spawner: &dyn Spawner) {
    state.workspace.message = Buffer::default();
    reread(state, spawner);
}
