//! The `workspace` controller: one function per user action on the `workspace` service.

mod diff;
mod editor;
mod git;
pub(crate) mod mr;
mod search;

use std::path::PathBuf;

use groove_types::{Edit, Selection, WorktreeId};

use self::diff::{mark_read, reread, show};
use self::editor::{copy, edit_file, open_file, paste, save_file};
use self::git::{Act, Remote, commit, discard_all, index, remote};
use self::search::grep;
use crate::{AppState, Services, Spawner};

pub use diff::{follow, load};
pub use mr::{known, poll, polls};
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `workspace.load`: the selected worktree's changed files.
    Load,
    /// `workspace.open_file`: one file's two sides and the rows between them.
    OpenFile {
        path: String,
        /// What the caret should hold once it is open, when the asking knows.
        at: Option<Selection>,
    },
    /// `workspace.mark_read`: one file read, or the mark taken off it.
    MarkRead { path: String },
    /// `workspace.grep`: every line holding this text, in the files `under` keeps.
    Grep { query: String, under: String },
    /// `workspace.fold`: one file's rows hidden under its head, or shown again.
    Fold { path: String },
    /// `workspace.show`: which rows of the whole change are on screen.
    Show { rows: std::ops::Range<usize> },
    /// `workspace.edit`: one keystroke on the open buffer.
    Edit(Edit),
    /// `workspace.save_file`: the buffer to the file it came from.
    SaveFile,
    /// `workspace.copy`: what the carets hold, to the clipboard.
    Copy,
    /// `workspace.cut`: the same, and out of the buffer.
    Cut,
    /// `workspace.paste`: the clipboard, over what the carets hold.
    Paste,
    /// `workspace.stage`: one path into the index.
    Stage { path: String },
    /// `workspace.unstage`: one path back out of it.
    Unstage { path: String },
    /// `workspace.discard`: what one path holds, thrown away.
    Discard { path: String },
    /// `workspace.message`: one keystroke on the commit message.
    Message(Edit),
    /// `workspace.commit`: the index, with the message the box holds.
    Commit,
    /// `workspace.push`: the branch to its own name on origin.
    Push,
    /// `workspace.pull`: origin's own head, fast-forward only.
    Pull,
    /// `workspace.discard_all`: every change in the worktree, thrown away.
    DiscardAll,
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "workspace.load",
            Command::OpenFile { .. } => "workspace.open_file",
            Command::MarkRead { .. } => "workspace.mark_read",
            Command::Grep { .. } => "workspace.grep",
            Command::Fold { .. } => "workspace.fold",
            Command::Show { .. } => "workspace.show",
            Command::Edit(_) => "workspace.edit",
            Command::SaveFile => "workspace.save_file",
            Command::Copy => "workspace.copy",
            Command::Cut => "workspace.cut",
            Command::Paste => "workspace.paste",
            Command::Stage { .. } => "workspace.stage",
            Command::Unstage { .. } => "workspace.unstage",
            Command::Discard { .. } => "workspace.discard",
            Command::Message(_) => "workspace.message",
            Command::Commit => "workspace.commit",
            Command::Push => "workspace.push",
            Command::Pull => "workspace.pull",
            Command::DiscardAll => "workspace.discard_all",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Load => reread(state, spawner),
        Command::OpenFile { path, at } => open_file(state, spawner, path, at),
        Command::MarkRead { path } => mark_read(state, services, spawner, path),
        Command::Grep { query, under } => grep(state, spawner, query, under),
        Command::Fold { path } => state.workspace.changes.fold(&path),
        Command::Show { rows } => show(state, spawner, rows),
        Command::Edit(edit) => edit_file(state, spawner, edit),
        Command::SaveFile => save_file(state, spawner),
        Command::Copy => copy(state, services, spawner, false),
        Command::Cut => copy(state, services, spawner, true),
        Command::Paste => paste(state, services, spawner),
        Command::Stage { path } => index(state, spawner, Act::Stage, path),
        Command::Unstage { path } => index(state, spawner, Act::Unstage, path),
        Command::Discard { path } => index(state, spawner, Act::Discard, path),
        Command::Message(edit) => state.workspace.message.edit(&edit),
        Command::Commit => commit(state, spawner),
        Command::Push => remote(state, spawner, Remote::Push),
        Command::Pull => remote(state, spawner, Remote::Pull),
        Command::DiscardAll => discard_all(state, spawner),
    }
}

/// Where the selected worktree sits on disk.
pub(super) fn worktree_dir(state: &AppState) -> Option<PathBuf> {
    let open = state.session.selected()?;
    Some(PathBuf::from(&open.selected_worktree()?.path))
}

pub(super) fn directory(state: &AppState, worktree: &WorktreeId) -> Option<PathBuf> {
    let open = state.session.selected()?;
    let found = open.worktrees.iter().find(|w| &w.id == worktree)?;
    Some(PathBuf::from(&found.path))
}

pub fn stale(state: &AppState) -> bool {
    match selected(state) {
        Some(worktree) => !state.workspace.holds(&worktree),
        None => false,
    }
}

pub(super) fn selected(state: &AppState) -> Option<WorktreeId> {
    let open = state.session.selected()?;
    Some(open.selected_worktree()?.id.clone())
}

pub fn loaded_for(state: &AppState) -> Option<&WorktreeId> {
    state.workspace.worktree.as_ref()
}
