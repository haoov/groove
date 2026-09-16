//! The workspace capability. Its slice of `AppState`, the operations on it, its events.

use std::path::Path;

use groove_types::{DiffMode, DiffView, FileDiff, Result, WorktreeId, WorktreeStatus};

/// What the workspace holds for the selected worktree.
#[derive(Debug, Default)]
pub struct State {
    /// Which worktree the summary below belongs to.
    pub worktree: Option<WorktreeId>,
    pub mode: DiffMode,
    pub view: DiffView,
    pub status: Option<WorktreeStatus>,
    pub files: Vec<FileDiff>,
}

impl State {
    /// Whether what is loaded is this worktree's.
    pub fn holds(&self, worktree: &WorktreeId) -> bool {
        self.worktree.as_ref() == Some(worktree)
    }

    pub fn loaded(&mut self, worktree: WorktreeId, files: Vec<FileDiff>) {
        self.worktree = Some(worktree);
        self.files = files;
    }

    /// The changed files, and nothing at all when they are another worktree's.
    pub fn files_of(&self, worktree: Option<&WorktreeId>) -> &[FileDiff] {
        match worktree {
            Some(id) if self.holds(id) => &self.files,
            _ => &[],
        }
    }

    /// Forgets what was loaded: nothing is selected, so nothing is the truth.
    pub fn clear(&mut self) {
        self.worktree = None;
        self.files.clear();
        self.status = None;
    }
}

/// Every changed file of the worktree at `dir`, with its counts.
pub async fn summary(dir: &Path) -> Result<Vec<FileDiff>> {
    groove_diff::summary(dir).await
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
