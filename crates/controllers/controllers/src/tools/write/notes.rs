//! The agent's notes and thread answers, built as the surface builds them.

use groove_types::{Anchor, AnnotationId};

use super::Write;
use crate::asker::Asker;
use crate::delivery::{NoteAct, ThreadAct, Whose, notes, threads};
use crate::tools::NO_WORKTREE;
use crate::{AppState, Services, Spawner};

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
    let anchor = Anchor {
        path,
        start_line,
        end_line,
    };
    let act = NoteAct::Create { anchor, content };
    noted(state, services, spawner, write, act);
}

/// The lines the note stands on, as the file numbers them.
fn lines(write: &Write) -> Option<(u32, u32)> {
    let line = |one: i64| u32::try_from(one.max(1) - 1).ok();
    let start = line(write.number("line")?)?;
    let end = match write.number("end_line") {
        Some(one) => line(one)?,
        None => start,
    };
    Some((start.min(end), start.max(end)))
}

/// A note's words written again.
pub(super) fn update(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(id) = note_id(&write) else {
        return write.reply.failed("update_annotation needs the note's id");
    };
    let Some(content) = write.text("content").map(str::to_string) else {
        return write.reply.failed("update_annotation needs the new words");
    };
    noted(
        state,
        services,
        spawner,
        write,
        NoteAct::Update { id, content },
    );
}

/// A note marked dealt with.
pub(super) fn resolve(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(id) = note_id(&write) else {
        return write.reply.failed("resolve_annotation needs the note's id");
    };
    noted(state, services, spawner, write, NoteAct::Resolve { id });
}

/// One note of this session posted on the MR, at its own line.
pub(super) fn post(state: &mut AppState, services: &Services, spawner: &dyn Spawner, write: Write) {
    let Some(id) = note_id(&write) else {
        return write.reply.failed("post_annotation needs the note's id");
    };
    said(state, services, spawner, write, ThreadAct::Post { id });
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

fn note_id(write: &Write) -> Option<AnnotationId> {
    write.text("id").map(AnnotationId::new)
}

fn thread_id(write: &Write) -> Option<String> {
    write.text("thread").map(str::to_string)
}

fn noted(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    act: NoteAct,
) {
    let Some(whose) = whose(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    notes::write(
        state,
        services,
        spawner,
        act,
        whose,
        Asker::Agent(write.reply),
    );
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
