//! Worktrees: the clone pool, provisioning for every session kind, naming,
//! and teardown.

pub mod naming;
mod ops;
mod pool;
mod provision;
mod status;
mod teardown;

// Glob re-exports are required: tauri::generate_handler! looks up __cmd__*
// symbols at the same path as the function.
pub use ops::*;
pub use pool::*;
pub use provision::*;
pub use status::*;
pub use teardown::*;

/// Drop every cached git answer, so the refresh that follows is exact.
/// The explicit-refresh half of the contract: callers refresh, this forgets.
#[tauri::command]
pub fn flush_git_caches() {
    crate::core::git::cache::flush();
}
