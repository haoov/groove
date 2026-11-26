//! What the agent says on the MR's threads, built as the surface builds it.

use groove_delivery_service::BY_AGENT;
use groove_types::Anchor;

use super::Write;
use crate::asker::Asker;
use crate::delivery::{ThreadAct, Whose, threads};
use crate::tools::NO_WORKTREE;
use crate::{AppState, Services, Spawner};

/// A new thread on the lines the write names, its words marked as the agent's.
pub(super) fn comment(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (write, path, body): (Write, String, &str),
) {
    let numbered = write
        .number("line")
        .map(|line| (line, write.number("end_line")));
    let Some(anchor) = numbered.and_then(|(line, end)| Anchor::numbered(path, line, end)) else {
        return write.reply.failed("comment_mr needs a line with its path");
    };
    let body = groove_types::decorated(body, BY_AGENT);
    said(
        state,
        services,
        spawner,
        write,
        ThreadAct::Comment { anchor, body },
    );
}

/// An answer under a thread of the MR.
pub(super) fn reply(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let (Some(thread), Some(body)) = (thread_id(&write), write.text("body")) else {
        return write
            .reply
            .failed("reply_thread needs the thread's id and a body");
    };
    let body = body.to_string();
    said(
        state,
        services,
        spawner,
        write,
        ThreadAct::Reply { thread, body },
    );
}

/// A thread of the MR resolved, or opened again.
pub(super) fn resolve_thread(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(thread) = thread_id(&write) else {
        return write.reply.failed("resolve_thread needs the thread's id");
    };
    let resolve = write.flag("resolve").unwrap_or(true);
    said(
        state,
        services,
        spawner,
        write,
        ThreadAct::Resolve { thread, resolve },
    );
}

fn thread_id(write: &Write) -> Option<String> {
    write.text("thread").map(str::to_string)
}

fn said(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    act: ThreadAct,
) {
    let Some(whose) = whose(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    threads::thread(
        state,
        services,
        spawner,
        act,
        whose,
        Asker::Agent(write.reply),
    );
}

/// The worktree the write names, with the session and repo that hold it.
pub(super) fn whose(state: &AppState, write: &Write) -> Option<Whose> {
    Whose::of(state, &super::worktree_of(state, write)?.id)
}
