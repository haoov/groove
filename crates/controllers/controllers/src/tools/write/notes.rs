//! What the agent leaves on a line, and what it says in the merge request's threads.
//! Each one builds the act the surface builds, and hands it to the same write.

use groove_types::{Anchor, AnnotationId};

use super::Write;
use crate::asker::Asker;
use crate::tools::NO_WORKTREE;
use crate::workspace::notes::{self, Act, Whose};
use crate::{AppState, Services, Spawner};

/// The agent's own name on the notes it leaves.
const AUTHOR: &str = "agent";

/// A note on a line, or on a range of them.
pub(super) fn create(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(path) = write.text("path").map(str::to_string) else {
        return write.reply.failed("create_annotation needs a path");
    };
    let Some(content) = write.text("content").map(str::to_string) else {
        return write
            .reply
            .failed("create_annotation needs the note itself");
    };
    let Some((start_line, end_line)) = lines(&write) else {
        return write.reply.failed("create_annotation needs a line");
    };
    let act = Act::Create {
        anchor: Anchor {
            path,
            start_line,
            end_line,
        },
        content,
        author: AUTHOR.to_string(),
    };
    made(state, services, spawner, write, act);
}

/// The lines the note stands on, as the file numbers them.
fn lines(write: &Write) -> Option<(u32, u32)> {
    let start = write.number("line")?.max(1) as u32 - 1;
    let end = write
        .number("end_line")
        .map(|one| one.max(1) as u32 - 1)
        .unwrap_or(start);
    Some((start.min(end), start.max(end)))
}

/// A note's words written again, or the note marked dealt with.
pub(super) fn on_note(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    resolving: bool,
) {
    let Some(id) = write.text("id").map(AnnotationId::new) else {
        return write.reply.failed("that tool needs the note's id");
    };
    let act = match (resolving, write.text("content")) {
        (true, _) => Act::Resolve { id },
        (false, Some(content)) => Act::Update {
            id,
            content: content.to_string(),
        },
        (false, None) => return write.reply.failed("update_annotation needs the new words"),
    };
    made(state, services, spawner, write, act);
}

/// One note of this session posted on the merge request, at its own line.
pub(super) fn post(state: &mut AppState, services: &Services, spawner: &dyn Spawner, write: Write) {
    let Some(id) = write.text("id").map(AnnotationId::new) else {
        return write.reply.failed("post_annotation needs the note's id");
    };
    made(state, services, spawner, write, Act::Post { id });
}

/// An answer under a thread of the merge request, or a thread resolved.
pub(super) fn thread(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    replying: bool,
) {
    let Some(thread) = write.text("thread").map(str::to_string) else {
        return write.reply.failed("that tool needs the thread's id");
    };
    let act = match (replying, write.text("body")) {
        (true, Some(body)) => Act::Reply {
            thread,
            body: body.to_string(),
        },
        (true, None) => return write.reply.failed("reply_thread needs a body"),
        (false, _) => Act::Thread {
            thread,
            resolve: write.flag("resolve").unwrap_or(true),
        },
    };
    made(state, services, spawner, write, act);
}

/// The act, against the worktree the write belongs to.
fn made(state: &mut AppState, services: &Services, spawner: &dyn Spawner, write: Write, act: Act) {
    let Some(whose) = whose(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let asker = Asker::Agent(write.reply);
    notes::write(state, services, spawner, act, whose, asker);
}

/// The session the write comes from, and the worktree its note stands on.
pub(super) fn whose(state: &AppState, write: &Write) -> Option<Whose> {
    let named = super::worktree_of(state, write)?;
    let (repo, worktree) = crate::workspace::pair(state, &named.id)?;
    Some(Whose {
        session: write.session.clone(),
        repo,
        worktree: worktree.id,
    })
}
