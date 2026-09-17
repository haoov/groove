//! The workspace capability. Its slice of `AppState`, the operations on it, its events.

use std::path::{Path, PathBuf};

pub use groove_diff::{Derived, Document, Opened, columns, display_at, from_text, shown};
use groove_types::{DiffMode, DiffView, FileDiff, Result, WorktreeId, WorktreeStatus};
use groove_watch::{QUIET, Watch};

#[cfg(test)]
mod tests;

/// What the workspace holds for the selected worktree.
#[derive(Debug, Default)]
pub struct State {
    /// Which worktree the summary below belongs to.
    pub worktree: Option<WorktreeId>,
    pub mode: DiffMode,
    pub view: DiffView,
    pub status: Option<WorktreeStatus>,
    pub files: Vec<FileDiff>,
    /// The file the diff is showing, with both its sides.
    pub opened: Option<Opened>,
    pub watching: Option<WorktreeId>,
    /// The buffer revision a read of the colours and the rows is out for.
    pub deriving: Option<u64>,
    watch: Option<Watch>,
}

impl State {
    pub fn holds(&self, worktree: &WorktreeId) -> bool {
        self.worktree.as_ref() == Some(worktree)
    }

    pub fn loaded(&mut self, worktree: WorktreeId, files: Vec<FileDiff>) {
        self.worktree = Some(worktree);
        self.files = files;
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
        self.worktree = None;
        self.files.clear();
        self.opened = None;
        self.status = None;
        self.watching = None;
        self.deriving = None;
        self.watch = None;
    }
}

/// Watches `dir` until another worktree is watched or the state is cleared.
pub fn watch(
    state: &mut State,
    worktree: WorktreeId,
    dir: &Path,
    on_change: impl Fn(Vec<PathBuf>) + Send + 'static,
) -> Result<()> {
    state.watch = None;
    state.watching = None;
    let watch = groove_watch::watch(dir, QUIET, on_change)?;
    state.watching = Some(worktree);
    state.watch = Some(watch);
    Ok(())
}

pub async fn summary(dir: &Path) -> Result<Vec<FileDiff>> {
    groove_diff::summary(dir).await
}

/// One file of the worktree, both sides and the rows between them.
pub async fn opened(dir: &Path, path: &str) -> Result<Opened> {
    groove_diff::opened(dir, path).await
}

/// The same file, against the HEAD side already read for it.
pub fn reopened(dir: &Path, path: &str, old: Document) -> Opened {
    groove_diff::reopened(dir, path, old)
}

/// The colours and the alignment of text the buffer now holds.
pub fn derived(path: &str, old: &Document, text: &str) -> Derived {
    groove_diff::derived(path, old, text)
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
