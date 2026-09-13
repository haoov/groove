//! The workspace capability. Its slice of `AppState`, the operations on it, its events.

use groove_types::{DiffMode, DiffView, FileDiff, WorktreeId, WorktreeStatus};

/// What the workspace holds for the selected worktree.
#[derive(Debug, Default)]
pub struct State {
    pub worktree: Option<WorktreeId>,
    pub mode: DiffMode,
    pub view: DiffView,
    pub status: Option<WorktreeStatus>,
    pub files: Vec<FileDiff>,
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
