//! What git is asked to do: the index, a commit, a push, a pull, a change thrown away.
//! The surface and the agent ask through the same functions, and say who asked.

use std::path::PathBuf;

use groove_types::{TimelineKind, Worktree};
use groove_workspace_service::{Buffer, summary};

use super::diff::{load, reread};
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// What one action does against the remote or the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Remote {
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

    fn kind(self) -> TimelineKind {
        match self {
            Remote::Push => TimelineKind::Push,
            Remote::Pull => TimelineKind::Pull,
        }
    }

    fn said(self) -> &'static str {
        match self {
            Remote::Push => "pushed",
            Remote::Pull => "pulled",
        }
    }
}

const NOTHING_STAGED: &str =
    "nothing is staged: stage what this commit holds with git add, then call again";

/// The index committed, then every side of the diff read again.
pub(crate) fn commit(
    state: &mut AppState,
    spawner: &dyn Spawner,
    worktree: Worktree,
    message: String,
    asker: Asker,
) {
    if message.trim().is_empty() {
        return asker.refused("a commit needs a message");
    }
    let session = worktree.session.clone();
    let dir = PathBuf::from(&worktree.path);
    let job = state.begin("committing");
    let clears = asker.carries_a_box();
    spawner.spawn(Box::pin(async move {
        let done = match staged(&dir).await {
            true => groove_workspace_service::commit(&dir, message.trim()).await,
            false => Err(groove_types::Error::invalid(NOTHING_STAGED)),
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                let said = subject(&message);
                if done.is_ok() {
                    let kind = TimelineKind::Commit;
                    crate::timeline::log(services, spawner, &session, kind, &said, &worktree.id);
                    if clears {
                        state.workspace.message = Buffer::default();
                    }
                    reread(state, spawner);
                }
                asker.answer(state, done, || format!("committed {said}"));
            },
        ) as Continuation
    }));
}

/// Whether the index holds anything at all.
async fn staged(dir: &std::path::Path) -> bool {
    summary(dir)
        .await
        .unwrap_or_default()
        .iter()
        .any(|file| file.staged == Some(true))
}

/// One action against the remote. HEAD may move, so everything is read again.
pub(crate) fn remote(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: Worktree,
    act: Remote,
    asker: Asker,
) {
    let session = worktree.session.clone();
    let dir = PathBuf::from(&worktree.path);
    let job = state.begin(act.label());
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let done = match act {
            Remote::Push => groove_workspace_service::push(&dir, &worktree.branch).await,
            Remote::Pull => groove_workspace_service::pull(&dir).await,
        };
        let status = service.status(&worktree).await.ok();
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                let branch = worktree.branch.clone();
                if done.is_ok() {
                    let kind = act.kind();
                    crate::timeline::log(services, spawner, &session, kind, &branch, &worktree.id);
                    if act == Remote::Push {
                        state.delivery.poll.forget(&worktree.id);
                    }
                    if let (Some(open), Some(status)) = (state.session.get_mut(&session), status) {
                        open.told(&worktree.id, status);
                    }
                    reread(state, spawner);
                }
                asker.answer(state, done, || format!("{} {branch}", act.said()));
            },
        ) as Continuation
    }));
}

/// What a commit is known by: the first line of its message.
fn subject(message: &str) -> String {
    message
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Every change in the worktree, thrown away.
pub(super) fn discard_all(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(dir) = state
        .session
        .selected_worktree()
        .map(groove_types::Worktree::dir)
    else {
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
                    Err(e) => state.failed(e),
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
    let Some(dir) = state
        .session
        .selected_worktree()
        .map(groove_types::Worktree::dir)
    else {
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
                    Err(e) => state.failed(e),
                }
            },
        ) as Continuation
    }));
}
