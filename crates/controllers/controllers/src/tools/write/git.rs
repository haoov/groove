//! What the agent asks git for, on the worktree it names.

use super::Write;
use crate::asker::Asker;
use crate::tools::NO_WORKTREE;
use crate::workspace::git;
use crate::{AppState, Services, Spawner};

pub(super) use crate::workspace::git::Remote;

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
