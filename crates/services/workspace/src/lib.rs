//! The workspace capability. Its slice of `AppState`, the operations on it, its events.

use std::path::{Path, PathBuf};

pub use groove_diff::{
    Aligned, At, Changes, Derived, Document, Opened, aligned, columns, display_at, from_documents,
    from_text, shown,
};
pub use groove_editor::{Clipboard, Memory, clipboard};
pub use groove_grep::{Found, Search};
pub use groove_text::{Buffer, Colours};
use std::collections::BTreeMap;
use std::ops::Range;

use groove_types::{CommitEntry, DiffMode, FileDiff, Result, WorktreeId, WorktreeStatus};
use groove_watch::{QUIET, Watch};

mod buffer;
mod buffers;
mod git;
mod paths;
mod read;
mod rows;

#[cfg(test)]
mod tests;

pub use buffers::Buffers;
pub use git::{UNPUSHED_MAX, commit, discard, pull, push, stage, unpushed, unstage};
pub use paths::{PathOp, path_op};
pub use read::{
    COMMITS_MAX, FOUND_MAX, HEAD, PATHS_MAX, at_commit, base_rev, changes, commits, derived, grep,
    opened, opened_at, painted, paths, reopened, summary, summary_against,
};
pub use rows::Way;

/// What the workspace holds for the selected worktree.
#[derive(Debug, Default)]
pub struct State {
    /// Which worktree the summary below belongs to.
    pub worktree: Option<WorktreeId>,
    pub mode: DiffMode,
    pub status: Option<WorktreeStatus>,
    pub files: Vec<FileDiff>,
    /// Every changed file's rows, the whole change as one surface.
    pub changes: Changes,
    /// How many summary reads were asked for; only the last one lands.
    pub reads: u64,
    /// The worktree and mode a summary read is out for, and whether another is owed after it.
    pub loading: Option<(WorktreeId, DiffMode)>,
    pub again: bool,
    /// Each worktree's open files, kept while another worktree is selected.
    buffers: BTreeMap<WorktreeId, Buffers>,
    /// Both sides parsed, for the files whose rows are on screen.
    pub coloured: BTreeMap<String, Painted>,
    /// The rows the surface last drew.
    pub showing: Range<usize>,
    /// What the last search across the worktree has found so far.
    pub found: Vec<Found>,
    /// Every file of the worktree, for the path term to narrow by, and a count that moves with it.
    paths: Vec<FileDiff>,
    paths_stamp: u64,
    /// A walk of the worktree is out.
    pub walking: bool,
    /// That search, while it still runs.
    pub searching: Option<std::sync::Arc<Search>>,
    /// Bumped whenever a document is read again, for a cache to know.
    pub stamp: u64,
    /// What the commit box holds, typed on the same buffer as a file.
    pub message: Buffer,
    /// The branch's own commits, newest first.
    pub log: Vec<CommitEntry>,
    /// The worktree whose commits it holds, or has a read out for.
    pub logged: Option<WorktreeId>,
    /// The commit the surface shows instead of the working tree.
    pub commit: Option<CommitEntry>,
    pub watching: Option<WorktreeId>,
    /// The buffers a read of the colours and the rows is out for.
    pub deriving: std::collections::BTreeSet<(WorktreeId, String)>,
    watch: Option<Watch>,
}

impl State {
    pub fn holds(&self, worktree: &WorktreeId) -> bool {
        self.worktree.as_ref() == Some(worktree)
    }

    /// The documents it holds are not the ones it held.
    pub fn moved(&mut self) {
        self.stamp = self.stamp.wrapping_add(1);
    }

    pub fn paths(&self) -> &[FileDiff] {
        &self.paths
    }

    pub fn paths_stamp(&self) -> u64 {
        self.paths_stamp
    }

    pub fn set_paths(&mut self, paths: Vec<FileDiff>) {
        self.paths = paths;
        self.paths_stamp = self.paths_stamp.wrapping_add(1);
    }

    pub fn clear_paths(&mut self) {
        self.set_paths(Vec::new());
    }

    pub fn loaded(&mut self, worktree: WorktreeId, files: Vec<FileDiff>, changes: Changes) {
        self.moved();
        if !self.holds(&worktree) {
            self.clear_paths();
        }
        self.worktree = Some(worktree);
        self.files = files;
        let shut = self.changes.folds();
        self.changes = changes;
        self.changes.refold(shut);
        self.coloured.clear();
        self.showing = 0..0;
    }

    /// Whether what it shows is a commit, which nothing may write.
    pub fn readonly(&self) -> bool {
        self.commit.is_some()
    }

    /// Shuts every open file the path that is gone is, or holds.
    pub fn shut_if_gone(&mut self, path: &str) {
        let under = format!("{path}/");
        let gone: Vec<String> = self
            .held_paths()
            .into_iter()
            .filter(|one| one == path || one.starts_with(&under))
            .collect();
        for one in gone {
            self.close(&one);
        }
    }

    /// The open files that are among these paths.
    pub fn shows(&self, paths: &[PathBuf]) -> Vec<String> {
        let held = self.held_paths();
        held.into_iter()
            .filter(|open| paths.iter().any(|path| path.ends_with(open)))
            .collect()
    }

    fn held_paths(&self) -> Vec<String> {
        let open = self.buffers().map(Buffers::all).unwrap_or_default();
        open.iter().map(|one| one.path.clone()).collect()
    }

    /// The changed files, and nothing at all when they are another worktree's.
    pub fn files_of(&self, worktree: Option<&WorktreeId>) -> &[FileDiff] {
        match worktree {
            Some(id) if self.holds(id) => &self.files,
            _ => &[],
        }
    }

    pub fn dirty(&self) -> Vec<String> {
        self.buffers().map(Buffers::dirty).unwrap_or_default()
    }

    /// Forgets what was loaded and stops watching.
    pub fn clear(&mut self) {
        self.moved();
        self.stop();
        self.found.clear();
        self.clear_paths();
        self.walking = false;
        self.log.clear();
        self.logged = None;
        self.commit = None;
        self.worktree = None;
        self.files.clear();
        self.changes = Changes::default();
        self.coloured.clear();
        self.showing = 0..0;
        self.status = None;
        self.watching = None;
        self.deriving.clear();
        self.watch = None;
    }
}

/// A file's two sides parsed, for the rows in view to take their colours from.
#[derive(Debug)]
pub struct Painted {
    pub old: Document,
    pub new: Document,
}

impl State {
    /// The search across the worktree gives up, for a new one or for nothing.
    pub fn stop(&mut self) {
        if let Some(search) = self.searching.take() {
            search.stop();
        }
    }
}

/// Whether any of these paths is git's own state rather than a file of it.
pub fn moved_git(paths: &[PathBuf]) -> bool {
    paths
        .iter()
        .any(|path| path.components().any(|part| part.as_os_str() == ".git"))
}

/// Where git keeps the worktree's own state.
pub async fn git_dir(dir: &Path) -> Option<PathBuf> {
    groove_git::Git::at(dir).git_dir().await.ok()
}

/// Watches `dir` and git's own directory, until the state is cleared.
pub fn watch(
    state: &mut State,
    worktree: WorktreeId,
    dir: &Path,
    git: Option<PathBuf>,
    on_change: impl Fn(Vec<PathBuf>) + Send + 'static,
) -> Result<()> {
    state.watch = None;
    state.watching = None;
    let watch = groove_watch::watch(dir, git.into_iter().collect(), QUIET, on_change)?;
    state.watching = Some(worktree);
    state.watch = Some(watch);
    Ok(())
}

/// Writes the buffer to the file it came from.
pub fn save(dir: &Path, path: &str, text: &str) -> Result<()> {
    groove_editor::save(dir, path, text)
}

/// What the outside world tells this capability.
#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
