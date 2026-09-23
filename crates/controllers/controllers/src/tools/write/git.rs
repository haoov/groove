//! What the agent asks git for: a commit of the index, a push, a pull.

use std::path::PathBuf;

use groove_types::TimelineKind;
use groove_workspace_service::summary;

use super::Write;
use crate::tools::{NO_WORKTREE, logged, worktree_named};
use crate::{AppState, Continuation, Services, Spawner};

/// What one action does against the remote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Act {
    Push,
    Pull,
}

impl Act {
    fn kind(self) -> TimelineKind {
        match self {
            Act::Push => TimelineKind::Push,
            Act::Pull => TimelineKind::Pull,
        }
    }

    fn said(self) -> &'static str {
        match self {
            Act::Push => "pushed",
            Act::Pull => "pulled",
        }
    }
}

const NOTHING_STAGED: &str =
    "nothing is staged: stage what this commit holds with git add, then call again";

/// The index committed, and the worktree read again.
pub(super) fn commit(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let Some(worktree) = worktree_named(state, write.text("worktree_id")) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let Some(message) = write.text("message").map(str::to_string) else {
        return write.reply.failed("git_commit needs a message");
    };
    let (session, reply) = (write.session, write.reply);
    let dir = PathBuf::from(&worktree.path);
    spawner.spawn(Box::pin(async move {
        let done = match staged(&dir).await {
            true => groove_workspace_service::commit(&dir, message.trim()).await,
            false => Err(groove_types::Error::invalid(NOTHING_STAGED)),
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match done {
                Ok(()) => {
                    let said = subject(&message);
                    let kind = TimelineKind::Commit;
                    logged(services, spawner, &session, kind, &said, &worktree.id);
                    crate::workspace::follow(state, spawner);
                    reply.said(format!("committed {said}"));
                }
                Err(e) => reply.failed(e.message),
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

/// The branch pushed to its own name on origin, or pulled fast-forward.
pub(super) fn remote(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    act: Act,
) {
    let Some(worktree) = worktree_named(state, write.text("worktree_id")) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let (session, reply) = (write.session, write.reply);
    let (service, dir) = (services.session.clone(), PathBuf::from(&worktree.path));
    spawner.spawn(Box::pin(async move {
        let done = match act {
            Act::Push => groove_workspace_service::push(&dir, &worktree.branch).await,
            Act::Pull => groove_workspace_service::pull(&dir).await,
        };
        let told = service.status(&worktree).await.ok();
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match done {
                Ok(()) => {
                    let branch = worktree.branch.clone();
                    let kind = act.kind();
                    logged(services, spawner, &session, kind, &branch, &worktree.id);
                    if let (Some(open), Some(told)) = (state.session.get_mut(&session), told) {
                        open.told(&worktree.id, told);
                    }
                    crate::workspace::follow(state, spawner);
                    reply.said(format!("{} {branch}", act.said()));
                }
                Err(e) => reply.failed(e.message),
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
