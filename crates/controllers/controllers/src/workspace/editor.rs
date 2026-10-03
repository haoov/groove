//! The open file: its two sides, what a keystroke does to it, and what a save owes.

use std::path::Path;

use groove_types::{Edit, Error, ErrorKind, Result, Selection};
use groove_workspace_service::{Document, Opened, derived, opened, reopened};

use crate::{AppState, Continuation, Services, Spawner};

/// One keystroke on the active buffer; its rows move at once, the diff and the colours in a job.
pub(super) fn edit_file(state: &mut AppState, spawner: &dyn Spawner, edit: Edit) {
    if let Some(path) = state.workspace.edit(&edit) {
        derive(state, spawner, path);
    }
}

/// One buffer's colours and alignment read again, one read at a time, the last revision winning.
pub(super) fn derive(state: &mut AppState, spawner: &dyn Spawner, path: String) {
    let Some(worktree) = state.workspace.worktree.clone() else {
        return;
    };
    let Some(open) = state.workspace.buffer(&path) else {
        return;
    };
    let key = (worktree.clone(), path.clone());
    if state.workspace.deriving.contains(&key) {
        return;
    }
    let revision = open.new.revision();
    let (old, new) = (open.old.clone(), open.new.document().clone());
    state.workspace.deriving.insert(key.clone());
    spawner.spawn(Box::pin(async move {
        let read = derived(&path, &old, new);
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.workspace.deriving.remove(&key);
                state.workspace.derived((&worktree, &path), read, revision);
                let moved = state
                    .workspace
                    .buffer(&path)
                    .is_some_and(|open| open.new.revision() != revision);
                if moved && state.workspace.holds(&worktree) {
                    derive(state, spawner, path);
                }
            },
        ) as Continuation
    }));
}

/// Writes the active buffer out.
pub(super) fn save_file(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.workspace.active() else {
        return;
    };
    let Some(worktree) = state.session.selected_worktree() else {
        return;
    };
    let (id, dir) = (worktree.id.clone(), worktree.dir());
    let (path, text) = (open.path.clone(), open.new.text());
    let job = state.begin(format!("saving {path}"));
    spawner.spawn(Box::pin(async move {
        let written = groove_workspace_service::save(&dir, &path, &text);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match written {
                Ok(()) => state.workspace.saved(&id, &path),
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
}

/// What the carets hold, to the clipboard, off the main thread.
pub(super) fn copy(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let held = state
        .workspace
        .active()
        .map(|open| open.new.selected())
        .unwrap_or_default();
    copied(services, spawner, held);
}

/// The same, and out of the buffer.
pub(super) fn cut(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let held = state
        .workspace
        .active()
        .map(|open| open.new.selected())
        .unwrap_or_default();
    if !held.is_empty() {
        edit_file(state, spawner, Edit::Delete);
    }
    copied(services, spawner, held);
}

fn copied(services: &Services, spawner: &dyn Spawner, held: String) {
    if held.is_empty() {
        return;
    }
    let clipboard = services.clipboard.clone();
    spawner.spawn(Box::pin(async move {
        let written = clipboard.write(&held);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = written {
                state.failed(Error::new(ErrorKind::Io, e.to_string()));
            }
        }) as Continuation
    }));
}

/// The clipboard read in a job, then put in where the carets are.
pub(super) fn paste(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    if state.workspace.active().is_none() {
        return;
    }
    let clipboard = services.clipboard.clone();
    spawner.spawn(Box::pin(async move {
        let text = clipboard.read();
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if let Some(text) = text.filter(|text| !text.is_empty()) {
                    edit_file(state, spawner, Edit::Insert(text));
                }
            },
        ) as Continuation
    }));
}

/// Where the HEAD side of a reopened file comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Head {
    /// Read it again: a git command may have moved it.
    Read,
    /// Keep the one in hand: only the working side changed.
    Keep,
}

/// The open files read again after the worktree moved; unsaved edits are left alone.
pub(super) fn reopen(state: &mut AppState, spawner: &dyn Spawner, head: Head, paths: Vec<String>) {
    for path in paths {
        let Some(open) = state.workspace.buffer(&path).filter(|one| !one.new.dirty()) else {
            continue;
        };
        let old = match head {
            Head::Keep => Some(open.old.clone()),
            Head::Read => None,
        };
        read(state, spawner, path, old, (None, false));
    }
}

/// Every open file of the selected worktree.
pub(super) fn held(state: &AppState) -> Vec<String> {
    let open = state
        .workspace
        .buffers()
        .map(|one| one.all())
        .unwrap_or_default();
    open.iter().map(|one| one.path.clone()).collect()
}

/// The file made active: its buffer when it is open, else the stream's documents, else a read.
pub fn open_file(state: &mut AppState, spawner: &dyn Spawner, path: String, at: Option<Selection>) {
    let Some(worktree) = state.workspace.worktree.clone() else {
        return read(state, spawner, path, None, (at, true));
    };
    if state.workspace.buffer(&path).is_some() {
        return state.workspace.focus(&path, at);
    }
    if let Some(file) = state.workspace.in_hand(&path) {
        return state.workspace.arrived(&worktree, file, at, true);
    }
    read(state, spawner, path, None, (at, true));
}

pub(super) fn read(
    state: &mut AppState,
    spawner: &dyn Spawner,
    path: String,
    old: Option<Document>,
    (at, focus): (Option<Selection>, bool),
) {
    let Some(asked) = super::Asked::now(state) else {
        return;
    };
    let job = state.begin(format!("opening {path}"));
    spawner.spawn(Box::pin(async move {
        let dir = &asked.dir;
        let rev = groove_workspace_service::against(dir, asked.mode, asked.base.as_deref()).await;
        let file = sides(dir, &path, old, &rev).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            if !asked.holds(state) {
                return;
            }
            match file {
                Ok(file) => state.workspace.arrived(&asked.worktree, file, at, focus),
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
}

/// A file's tab taken away; the ui asked first when it owed the disk.
pub(super) fn close_file(state: &mut AppState, path: &str) {
    state.workspace.close(path);
}

/// The file's two sides, reading the old one only when it is not in hand.
async fn sides(dir: &Path, path: &str, old: Option<Document>, rev: &str) -> Result<Opened> {
    match old {
        Some(old) => Ok(reopened(dir, path, old)),
        None => opened(dir, path, rev).await,
    }
}
