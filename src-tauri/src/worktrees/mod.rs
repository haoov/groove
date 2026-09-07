//! Worktrees: the clone pool, provisioning for every session kind, naming,
//! and teardown.

pub mod naming;
mod ops;
mod pool;
mod provision;
mod status;
mod teardown;

// Keep the glob re-exports: `generate_handler!` resolves `__cmd__*` symbols at this path.
pub use ops::*;
pub use pool::*;
pub use provision::*;
pub use status::*;
pub use teardown::*;

/// Drop every cached git answer.
#[tauri::command]
pub fn flush_git_caches() {
    crate::core::git::cache::flush();
}
