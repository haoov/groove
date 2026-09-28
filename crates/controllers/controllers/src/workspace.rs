//! The `workspace` controller: one function per user action on the `workspace` service.

mod commits;
pub(crate) mod diff;
mod editor;
pub(crate) mod git;
mod paths;
mod search;

use crate::asker::Asker;
use groove_types::{DiffMode, Edit, Selection};

use self::diff::{mark_read, reread, show};
use self::editor::{close_file, copy, cut, edit_file, open_file, paste, save_file};
use self::git::{Act, Remote, discard_all, index};
use self::search::{grep, list_paths};
use crate::{AppState, Services, Spawner};

pub use diff::{follow, load};
pub use groove_workspace_service::Way;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `workspace.load`: the selected worktree's changed files.
    Load,
    /// `workspace.set_mode`: what the change is read against, then read it again.
    SetMode { mode: DiffMode },
    /// `workspace.open_file`: one file's two sides and the rows between them.
    OpenFile {
        path: String,
        /// What the caret should hold once it is open, when the asking knows.
        at: Option<Selection>,
    },
    /// `workspace.close_file`: one open file's tab taken away, unsaved edits and all.
    CloseFile { path: String },
    /// `workspace.mark_read`: one file read, or the mark taken off it.
    MarkRead { path: String },
    /// `workspace.grep`: every line holding this text, in the files `under` keeps.
    Grep { query: String, under: String },
    /// `workspace.fold`: one file's rows hidden under its head, or shown again.
    Fold { path: String },
    /// `workspace.open_gap`: a gap gives up the lines it hides, from one end or whole.
    OpenGap { row: usize, way: Way },
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
    /// `workspace.list_paths`: every file of the worktree, for the path term.
    ListPaths,
    /// `workspace.path`: one file or directory made, moved, copied or taken away.
    Path(groove_workspace_service::PathOp),
    /// `workspace.get_commits`: the newest commits of the worktree's branch.
    GetCommits,
    /// `workspace.open_commit`: one commit shown as the change it made.
    OpenCommit { sha: String },
    /// `workspace.leave_commit`: the working tree shown again.
    LeaveCommit,
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "workspace.load",
            Command::SetMode { .. } => "workspace.set_mode",
            Command::OpenFile { .. } => "workspace.open_file",
            Command::CloseFile { .. } => "workspace.close_file",
            Command::MarkRead { .. } => "workspace.mark_read",
            Command::Grep { .. } => "workspace.grep",
            Command::Fold { .. } => "workspace.fold",
            Command::OpenGap { .. } => "workspace.open_gap",
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
            Command::ListPaths => "workspace.list_paths",
            Command::Path(_) => "workspace.path",
            Command::GetCommits => "workspace.get_commits",
            Command::OpenCommit { .. } => "workspace.open_commit",
            Command::LeaveCommit => "workspace.leave_commit",
        }
    }
}

impl Command {
    /// Whether it writes what the surface shows, which a commit never allows.
    pub fn writes(&self) -> bool {
        matches!(
            self,
            Command::Edit(_)
                | Command::SaveFile
                | Command::Cut
                | Command::Paste
                | Command::Stage { .. }
                | Command::Unstage { .. }
                | Command::Discard { .. }
                | Command::DiscardAll
                | Command::Commit
                | Command::Path(_)
        )
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    if state.workspace.readonly() && command.writes() {
        return;
    }
    match command {
        Command::Load => reread(state, spawner),
        Command::SetMode { mode } => diff::set_mode(state, spawner, mode),
        Command::OpenFile { path, at } => open_file(state, spawner, path, at),
        Command::CloseFile { path } => close_file(state, &path),
        Command::MarkRead { path } => mark_read(state, services, spawner, path),
        Command::Grep { query, under } => grep(state, spawner, query, under),
        Command::Fold { path } => state.workspace.changes.fold(&path),
        Command::OpenGap { row, way } => state.workspace.open_gap_at(row, way),
        Command::Show { rows } => show(state, spawner, rows),
        Command::Edit(edit) => edit_file(state, spawner, edit),
        Command::SaveFile => save_file(state, spawner),
        Command::Copy => copy(state, services, spawner),
        Command::Cut => cut(state, services, spawner),
        Command::Paste => paste(state, services, spawner),
        Command::Stage { path } => index(state, spawner, Act::Stage, path),
        Command::Unstage { path } => index(state, spawner, Act::Unstage, path),
        Command::Discard { path } => index(state, spawner, Act::Discard, path),
        Command::Message(edit) => {
            state.workspace.message.edit(&edit);
        }
        Command::Commit => commit_here(state, spawner),
        Command::Push => on_remote(state, services, spawner, Remote::Push),
        Command::Pull => on_remote(state, services, spawner, Remote::Pull),
        Command::DiscardAll => discard_all(state, spawner),
        Command::ListPaths => list_paths(state, spawner),
        Command::Path(op) => paths::act(state, spawner, op),
        Command::GetCommits => commits::list(state, spawner),
        Command::OpenCommit { sha } => commits::open(state, spawner, sha),
        Command::LeaveCommit => commits::leave(state, spawner),
    }
}

/// What a read of the selected worktree is for; an answer that lands after it moved is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asked {
    pub worktree: groove_types::WorktreeId,
    pub dir: std::path::PathBuf,
    pub mode: DiffMode,
    pub base: Option<String>,
    pub commit: Option<String>,
}

impl Asked {
    pub(crate) fn now(state: &AppState) -> Option<Self> {
        let one = state.session.selected_worktree()?;
        Some(Self {
            worktree: one.id.clone(),
            dir: one.dir(),
            mode: state.workspace.mode,
            base: one.base_ref.clone(),
            commit: state.workspace.commit.as_ref().map(|at| at.sha.clone()),
        })
    }

    /// Whether the selection is still the one this was read for.
    pub(crate) fn holds(&self, state: &AppState) -> bool {
        Self::now(state).as_ref() == Some(self)
    }
}

pub fn stale(state: &AppState) -> bool {
    match state.session.selected_worktree().map(|one| one.id.clone()) {
        Some(worktree) => !state.workspace.holds(&worktree),
        None => false,
    }
}

/// The words of the commit box on the selected worktree, from the surface.
fn commit_here(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(worktree) = state.session.selected_worktree().cloned() else {
        return;
    };
    let message = state.workspace.message.text();
    git::commit(state, spawner, worktree, message, Asker::Ui);
}

/// A push or a pull of the selected worktree, from the surface.
fn on_remote(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: Remote) {
    let Some(worktree) = state.session.selected_worktree().cloned() else {
        return;
    };
    git::remote(state, services, spawner, worktree, act, Asker::Ui);
}
