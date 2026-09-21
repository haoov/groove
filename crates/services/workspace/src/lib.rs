//! The workspace capability. Its slice of `AppState`, the operations on it, its events.

use std::path::{Path, PathBuf};

pub use groove_diff::{
    Aligned, At, Changes, Derived, Document, Opened, Words, aligned, by_line, columns, display_at,
    from_text, shown,
};
pub use groove_editor::{Clipboard, Memory, clipboard};
pub use groove_forge::{Remote, Snapshot};
pub use groove_grep::{Found, Search};
pub use groove_text::{Buffer, Colours};
use std::collections::BTreeMap;
use std::ops::Range;

use groove_types::{
    DiffMode, DiffView, FileDiff, MrFacts, Result, ReviewMr, WorktreeId, WorktreeStatus,
};
use groove_watch::{QUIET, Watch};

mod delivery;
mod git;
mod propose;
mod read;
mod service;

#[cfg(test)]
mod tests;

pub use delivery::{Delivery, Polling};
pub use git::{commit, discard, pull, push, stage, unstage};
pub use propose::{Text, text_of};
pub use read::{
    FOUND_MAX, PATHS_MAX, changes, derived, grep, opened, painted, paths, reopened, summary,
};
pub use service::{Delivered, Service};

/// What the workspace holds for the selected worktree.
#[derive(Debug, Default)]
pub struct State {
    /// Which worktree the summary below belongs to.
    pub worktree: Option<WorktreeId>,
    pub mode: DiffMode,
    pub view: DiffView,
    pub status: Option<WorktreeStatus>,
    pub files: Vec<FileDiff>,
    /// Every changed file's rows, the whole change as one surface.
    pub changes: Changes,
    /// The file the diff is showing, with both its sides.
    pub opened: Option<Opened>,
    /// Both sides parsed, for the files whose rows are on screen.
    pub coloured: BTreeMap<String, Painted>,
    /// The rows the surface last drew.
    pub showing: Range<usize>,
    /// What the last search across the worktree has found so far.
    pub found: Vec<Found>,
    /// Every file of the worktree, for the path term to narrow by.
    pub paths: Vec<FileDiff>,
    /// That search, while it still runs.
    pub searching: Option<std::sync::Arc<Search>>,
    /// Bumped whenever a document is read again, for a cache to know.
    pub stamp: u64,
    /// What the commit box holds, typed on the same buffer as a file.
    pub message: Buffer,
    /// The MR of the selected worktree, and what the forge last said about it.
    pub delivery: Delivery,
    /// The poll's clock, and which worktrees it has asked about.
    pub poll: Polling,
    /// What the attention rules read of each worktree's MR.
    pub facts: BTreeMap<WorktreeId, MrFacts>,
    /// The board's review column: what the forges ask this user to look at.
    pub reviews: Vec<ReviewMr>,
    pub watching: Option<WorktreeId>,
    /// The buffer revision a read of the colours and the rows is out for.
    pub deriving: Option<u64>,
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

    pub fn loaded(&mut self, worktree: WorktreeId, files: Vec<FileDiff>, changes: Changes) {
        self.moved();
        self.worktree = Some(worktree);
        self.files = files;
        let shut = self.changes.folds();
        self.changes = changes;
        self.changes.refold(shut);
        if self
            .opened
            .as_ref()
            .is_some_and(|open| self.gone(&open.path))
        {
            self.opened = None;
        }
    }

    /// Whether the summary still holds this path.
    fn gone(&self, path: &str) -> bool {
        !self.files.iter().any(|file| file.path == path)
    }

    /// Whether any of these paths is git's own state rather than a file of it.
    pub fn moved_git(&self, paths: &[PathBuf]) -> bool {
        paths
            .iter()
            .any(|path| path.components().any(|part| part.as_os_str() == ".git"))
    }

    /// Whether the open file is one of these paths.
    pub fn shows(&self, paths: &[PathBuf]) -> bool {
        let Some(open) = self.opened.as_ref() else {
            return false;
        };
        paths.iter().any(|path| path.ends_with(&open.path))
    }

    /// The changed files, and nothing at all when they are another worktree's.
    pub fn files_of(&self, worktree: Option<&WorktreeId>) -> &[FileDiff] {
        match worktree {
            Some(id) if self.holds(id) => &self.files,
            _ => &[],
        }
    }

    /// What the open file owes the disk.
    pub fn dirty(&self) -> bool {
        self.opened.as_ref().is_some_and(|open| open.new.dirty())
    }

    /// Forgets what was loaded and stops watching.
    pub fn clear(&mut self) {
        self.moved();
        self.stop();
        self.found.clear();
        self.paths.clear();
        self.facts.clear();
        self.worktree = None;
        self.files.clear();
        self.changes = Changes::default();
        self.coloured.clear();
        self.showing = 0..0;
        self.opened = None;
        self.status = None;
        self.watching = None;
        self.delivery.none();
        self.deriving = None;
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
    /// The paths whose rows `rows` covers.
    pub fn over(&self, rows: Range<usize>) -> Vec<String> {
        let mut paths: Vec<String> = Vec::new();
        for row in rows {
            let path = match self.changes.at(row) {
                Some(At::Band(file) | At::Head(file) | At::Row(file, _)) => &file.path,
                None => break,
            };
            if paths.last().is_some_and(|last| last == path) {
                continue;
            }
            paths.push(path.clone());
        }
        paths
    }

    /// The colours of one file, from the buffer when it is the open one.
    pub fn sides(&self, path: &str) -> Option<(&Document, &Document)> {
        if let Some(open) = self.opened.as_ref().filter(|open| open.path == path) {
            return Some((&open.old, open.new.document()));
        }
        let painted = self.coloured.get(path)?;
        Some((&painted.old, &painted.new))
    }
}

impl State {
    /// The search across the worktree gives up, for a new one or for nothing.
    pub fn stop(&mut self) {
        if let Some(search) = self.searching.take() {
            search.stop();
        }
    }
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

/// The filesystem watcher and the forge poll speak here.
#[derive(Debug)]
pub enum Event {
    FilesChanged {
        worktree: WorktreeId,
        paths: Vec<String>,
    },
}

pub fn apply(_state: &mut State, event: Event) {
    match event {
        Event::FilesChanged { .. } => {}
    }
}
