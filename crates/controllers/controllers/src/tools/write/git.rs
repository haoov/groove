//! What the agent asks git for, on the worktree it names.

use super::Write;
use crate::asker::Asker;
use crate::tools::NO_WORKTREE;
use crate::workspace::git;
use crate::{AppState, Services, Spawner};

pub(super) use crate::workspace::git::Remote;

/// A push waits with its branch and the commits it sends, read before it is queued.
pub(super) fn push_asked(state: &mut AppState, spawner: &dyn Spawner, mut write: Write) {
    let Some(worktree) = super::worktree_of(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    spawner.spawn(Box::pin(async move {
        let dir = std::path::Path::new(&worktree.path);
        let base = worktree.base_ref.as_deref();
        let commits = groove_workspace_service::unpushed(dir, &worktree.branch, base)
            .await
            .unwrap_or_default();
        let lines: Vec<String> = commits
            .iter()
            .map(|one| {
                format!(
                    "{} {}",
                    one.short_sha,
                    one.message.lines().next().unwrap_or_default()
                )
            })
            .collect();
        write.arguments["worktree_id"] = worktree.id.as_str().into();
        write.arguments["branch"] = worktree.branch.clone().into();
        write.arguments["commits"] = lines.into();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            match state.agent.agent(&write.session) {
                Some(_) => super::queued(state, write),
                None => write.reply.failed("the session closed before git_push ran"),
            }
        }) as crate::Continuation
    }));
}

/// The index of one worktree committed.
pub(super) fn commit(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let Some(worktree) = super::worktree_of(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let Some(message) = write.text("message").map(str::to_string) else {
        return write.reply.failed("git_commit needs a message");
    };
    git::commit(state, spawner, worktree, message, Asker::Agent(write.reply));
}

/// One worktree's branch pushed to its own name on origin, or pulled.
pub(super) fn remote(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    act: Remote,
) {
    let Some(worktree) = super::worktree_of(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    git::remote(
        state,
        services,
        spawner,
        worktree,
        act,
        Asker::Agent(write.reply),
    );
}
