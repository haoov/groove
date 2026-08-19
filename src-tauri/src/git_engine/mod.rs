mod blame;
mod commits;
mod diff;
mod ops;
mod parse;
mod status;
mod types;
mod watcher;
mod worktrees;

// Glob re-exports are required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function, so they must all be available at git_engine::*.
pub use blame::*;
pub use commits::*;
pub use diff::*;
pub use ops::*;
pub use status::*;
// The DTOs the commands return; other modules construct BranchSpec / read MainRepo.
pub use types::*;
pub use watcher::*;
pub use worktrees::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct State {
    pub(crate) inner: Arc<StateInner>,
}

pub(crate) struct StateInner {
    // worktree path → notify watcher, kept alive for the lifetime of the session.
    // Keyed by path so re-opening a session doesn't spawn duplicate watchers.
    pub(crate) watchers: Mutex<HashMap<String, notify::RecommendedWatcher>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                watchers: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
