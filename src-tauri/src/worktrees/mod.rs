//! Worktrees: the clone pool, provisioning for every session kind, naming,
//! teardown, and the filesystem watcher.

pub mod naming;
mod ops;
mod pool;
mod provision;
mod status;
mod teardown;
mod watcher;

// Glob re-exports are required: tauri::generate_handler! looks up __cmd__*
// symbols at the same path as the function.
pub use ops::*;
pub use pool::*;
pub use provision::*;
pub use status::*;
pub use teardown::*;
pub use watcher::*;

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
