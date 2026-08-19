//! Reading changes: diffs, blame, commit history, and the parsers under them.

mod blame;
mod commits;
mod diff;
mod parse;
pub mod types;

// Glob re-exports are required: tauri::generate_handler! looks up __cmd__*
// symbols at the same path as the function.
pub use blame::*;
pub use commits::*;
pub use diff::*;
