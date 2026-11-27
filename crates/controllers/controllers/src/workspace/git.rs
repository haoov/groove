//! What git is asked to do: a commit, a push, a pull.

use std::path::PathBuf;

use groove_types::{TimelineKind, Worktree};

use super::diff::reread;
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

/// The index committed, then every side of the diff read again.
pub(crate) fn commit(
    state: &mut AppState,
    spawner: &dyn Spawner,
    worktree: Worktree,
    message: String,
    asker: Asker,
) {
    if message.trim().is_empty() {
        return asker.refused(state, "a commit needs a message");
    }
    let session = worktree.session.clone();
    let dir = PathBuf::from(&worktree.path);
    let job = state.begin("committing");
    spawner.spawn(Box::pin(async move {
        let done = groove_workspace_service::commit(&dir, &message).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                let said = groove_types::subject_of(&message).to_string();
                if done.is_ok() {
                    let kind = TimelineKind::Commit;
                    crate::timeline::log(services, spawner, &session, kind, &said, &worktree.id);
                    reread(state, spawner);
                }
                asker.answer(state, done, || format!("committed {said}"));
            },
        ) as Continuation
    }));
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
