//! The `workspace` controller: one function per user action on the `workspace` service.

mod commits;
mod diff;
mod editor;
pub(crate) mod forge;
mod git;
pub(crate) mod mr;
pub(crate) mod notes;
mod paths;
pub(crate) mod queue;
mod search;
mod write;

use std::path::PathBuf;

use groove_types::{DiffMode, Edit, Selection, WorktreeId};

use self::diff::{mark_read, reread, show};
use self::editor::{copy, edit_file, open_file, paste, save_file};
use self::git::{Act, Remote, commit, discard_all, index, remote};
use self::search::{grep, list_paths};
use self::write::{Act as Mr, write};
use crate::{AppState, Services, Spawner};

pub use diff::{follow, load};
pub use forge::Say;
pub use mr::{known, poll, polls, refresh};
pub use notes::Act as NoteAct;
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
    /// `workspace.refresh_mr`: the selected worktree's MR, read again now.
    RefreshMr,
    /// `workspace.list_paths`: every file of the worktree, for the path term.
    ListPaths,
    /// `workspace.review_queue`: the MRs the forges ask this user to review.
    ReviewQueue,
    /// `workspace.path`: one file or directory made, moved, copied or taken away.
    Path(groove_workspace_service::PathOp),
    /// `workspace.create_mr`: the worktree's branch offered to its base.
    CreateMr,
    /// `workspace.update_mr`: its title and body written again from the box.
    UpdateMr,
    /// `workspace.close_mr`: closed, with nothing merged.
    CloseMr,
    /// `workspace.get_commits`: the newest commits of the worktree's branch.
    GetCommits,
    /// `workspace.open_commit`: one commit shown as the change it made.
    OpenCommit { sha: String },
    /// `workspace.leave_commit`: the working tree shown again.
    LeaveCommit,
    /// `workspace.get_notes`: this session's notes and the MR's threads.
    GetNotes,
    /// One note of this session made, written again, resolved or taken away.
    Note(NoteAct),
    /// What the commit box says on the merge request: a comment, or a verdict.
    Say(Say),
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "workspace.load",
            Command::SetMode { .. } => "workspace.set_mode",
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
            Command::RefreshMr => "workspace.refresh_mr",
            Command::ListPaths => "workspace.list_paths",
            Command::ReviewQueue => "workspace.review_queue",
            Command::Path(_) => "workspace.path",
            Command::CreateMr => "workspace.create_mr",
            Command::UpdateMr => "workspace.update_mr",
            Command::CloseMr => "workspace.close_mr",
            Command::GetCommits => "workspace.get_commits",
            Command::OpenCommit { .. } => "workspace.open_commit",
            Command::LeaveCommit => "workspace.leave_commit",
            Command::GetNotes => "workspace.get_notes",
            Command::Note(act) => act.id(),
            Command::Say(say) => say.id(),
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
                | Command::Note(_)
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
        Command::RefreshMr => mr::refresh(state, services, spawner),
        Command::ListPaths => list_paths(state, spawner),
        Command::ReviewQueue => queue::read(state, services, spawner),
        Command::Path(op) => paths::act(state, spawner, op),
        Command::CreateMr => write(state, services, spawner, Mr::Open),
        Command::UpdateMr => write(state, services, spawner, Mr::Edit),
        Command::CloseMr => write(state, services, spawner, Mr::Close),
        Command::GetCommits => commits::list(state, spawner),
        Command::OpenCommit { sha } => commits::open(state, spawner, sha),
        Command::LeaveCommit => commits::leave(state, spawner),
        Command::GetNotes => notes::list(state, services, spawner),
        Command::Note(act) => notes::write(state, services, spawner, act),
        Command::Say(one) => forge::say(state, services, spawner, one),
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

/// The branch the selected worktree merges into.
pub(super) fn selected_base(state: &AppState) -> Option<String> {
    state
        .session
        .selected()?
        .selected_worktree()?
        .base_ref
        .clone()
}

pub(super) fn selected(state: &AppState) -> Option<WorktreeId> {
    let open = state.session.selected()?;
    Some(open.selected_worktree()?.id.clone())
}

/// The repo and the worktree one id names, in whichever open session holds it.
pub(super) fn pair(
    state: &AppState,
    id: &WorktreeId,
) -> Option<(groove_types::Repo, groove_types::Worktree)> {
    for open in state.session.open.iter() {
        let Some(worktree) = open.worktrees.iter().find(|one| &one.id == id) else {
            continue;
        };
        let repo = open.repos.iter().find(|repo| repo.id == worktree.repo)?;
        return Some((repo.clone(), worktree.clone()));
    }
    None
}

pub fn loaded_for(state: &AppState) -> Option<&WorktreeId> {
    state.workspace.worktree.as_ref()
}
