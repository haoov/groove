//! The open file: its two sides, what a keystroke does to it, and what a save owes.

use std::path::Path;

use groove_types::{Edit, Error, ErrorKind, Result, Selection};
use groove_workspace_service::{
    Derived, Document, Opened, by_line, derived, from_documents, opened, opened_at, reopened,
};

use super::worktree_dir;
use crate::{AppState, Continuation, Services, Spawner};

/// One keystroke on the buffer; its rows and colours follow in a job.
pub(super) fn edit_file(state: &mut AppState, spawner: &dyn Spawner, edit: Edit) {
    let Some(open) = state.workspace.opened.as_mut() else {
        return;
    };
    let before = open.new.revision();
    open.new.edit(&edit);
    if open.new.revision() != before {
        derive(state, spawner);
    }
}

/// Reads the colours and the alignment again, one read at a time: the last
/// revision wins, and a read that lands stale starts the next one.
pub(super) fn derive(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    if state.workspace.deriving.is_some() {
        return;
    }
    let revision = open.new.revision();
    let (path, old, new) = (
        open.path.clone(),
        open.old.clone(),
        open.new.document().clone(),
    );
    state.workspace.deriving = Some(revision);
    spawner.spawn(Box::pin(async move {
        let read = derived(&path, &old, new);
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.workspace.deriving = None;
                took(state, path, read, revision);
                if state
                    .workspace
                    .opened
                    .as_ref()
                    .is_some_and(|open| open.new.revision() != revision)
                {
                    derive(state, spawner);
                }
            },
        ) as Continuation
    }));
}

/// Installs what the read found, when the buffer is still the one it read.
pub(super) fn took(state: &mut AppState, path: String, read: Derived, revision: u64) {
    let Some(open) = state.workspace.opened.as_mut() else {
        return;
    };
    if open.path != path || !open.new.settled(read.settled, revision) {
        return;
    }
    open.rows = read.aligned.rows.clone();
    open.marks = read.aligned.marks.clone();
    open.words = by_line(&read.aligned.rows, &read.aligned.words);
    state.workspace.changes.replace(read.aligned);
}

/// Writes the buffer out. The watcher's read of our own write finds it clean.
pub(super) fn save_file(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let (path, text) = (open.path.clone(), open.new.text());
    let job = state.begin(format!("saving {path}"));
    spawner.spawn(Box::pin(async move {
        let written = groove_workspace_service::save(&dir, &path, &text);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match written {
                Ok(()) => saved(state, &path),
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
}

/// The buffer owes the disk nothing, so a later read may replace it.
pub(super) fn saved(state: &mut AppState, path: &str) {
    if let Some(open) = state
        .workspace
        .opened
        .as_mut()
        .filter(|open| open.path == path)
    {
        open.new.saved();
    }
}

/// What the carets hold, to the clipboard, off the main thread.
pub(super) fn copy(state: &mut AppState, services: &Services, spawner: &dyn Spawner, cut: bool) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    let held = open.new.selected();
    if held.is_empty() {
        return;
    }
    if cut {
        edit_file(state, spawner, Edit::Delete);
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
    if state.workspace.opened.is_none() {
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

/// Reads the open file again, now the worktree has moved under it. A buffer with
/// unsaved edits is left alone: the user's text outranks the disk's.
pub(super) fn reopen(state: &mut AppState, spawner: &dyn Spawner, head: Head) {
    let Some(open) = state.workspace.opened.as_ref() else {
        return;
    };
    if open.new.dirty() {
        return;
    }
    let path = open.path.clone();
    let old = match head {
        Head::Keep => Some(open.old.clone()),
        Head::Read => None,
    };
    read(state, spawner, path, old, None);
}

/// The file opened from the documents the stream holds, or read in a job when it has none.
pub fn open_file(state: &mut AppState, spawner: &dyn Spawner, path: String, at: Option<Selection>) {
    if let Some(file) = in_hand(state, &path) {
        return arrived(state, file, at);
    }
    read(state, spawner, path, None, at);
}

/// The two sides of a file the stream holds, aligned again for the buffer.
fn in_hand(state: &AppState, path: &str) -> Option<Opened> {
    if state.workspace.commit.is_some() {
        return None;
    }
    let painted = state.workspace.coloured.get(path)?;
    Some(from_documents(
        path,
        painted.old.clone(),
        painted.new.clone(),
    ))
}

pub(super) fn read(
    state: &mut AppState,
    spawner: &dyn Spawner,
    path: String,
    old: Option<Document>,
    at: Option<Selection>,
) {
    let Some(dir) = worktree_dir(state) else {
        return;
    };
    let job = state.begin(format!("opening {path}"));
    let sha = state.workspace.commit.as_ref().map(|one| one.sha.clone());
    let mode = state.workspace.mode;
    let base = super::selected_base(state);
    spawner.spawn(Box::pin(async move {
        let rev = super::diff::against(&dir, mode, base.as_deref()).await;
        let file = match sha {
            Some(sha) => opened_at(&dir, &sha, &path).await,
            None => sides(&dir, &path, old, &rev).await,
        };
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match file {
                Ok(file) => arrived(state, file, at),
                Err(e) => state.failed(e),
            }
        }) as Continuation
    }));
}

/// The file read, with the caret it had while it was the same file.
pub(super) fn arrived(state: &mut AppState, file: Opened, at: Option<Selection>) {
    match reads_the_same(state, &file) {
        true => refreshed(state, file, at),
        false => replaced(state, file, at),
    }
}

/// Whether the buffer in hand already holds the text this read found.
fn reads_the_same(state: &AppState, file: &Opened) -> bool {
    state
        .workspace
        .opened
        .as_ref()
        .is_some_and(|open| open.path == file.path && open.new.text() == file.new.text())
}

/// The read brought back the text the buffer holds: the buffer stays, with its history.
fn refreshed(state: &mut AppState, file: Opened, at: Option<Selection>) {
    let Some(open) = state.workspace.opened.as_mut() else {
        return;
    };
    open.old = file.old;
    open.rows = file.rows;
    open.marks = file.marks;
    open.words = file.words;
    open.long = file.long;
    open.new.saved();
    if let Some(held) = at {
        open.new.holding(held);
    }
}

/// A different text: the buffer gives way, keeping only where the caret was.
fn replaced(state: &mut AppState, mut file: Opened, at: Option<Selection>) {
    let held = at.or_else(|| {
        state
            .workspace
            .opened
            .as_ref()
            .filter(|open| open.path == file.path)
            .map(|open| Selection::at(open.new.caret()))
    });
    if let Some(held) = held {
        file.new.holding(held);
    }
    state.workspace.opened = Some(file);
    state.workspace.moved();
}

/// The file's two sides, reading the old one only when it is not in hand.
async fn sides(dir: &Path, path: &str, old: Option<Document>, rev: &str) -> Result<Opened> {
    match old {
        Some(old) => Ok(reopened(dir, path, old)),
        None => opened(dir, path, rev).await,
    }
}
