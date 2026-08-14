mod commands;
mod github;
mod gitlab;
mod ops;
mod platform;

// Glob re-export is required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function, so they must be re-exported from this module too.
pub use commands::*;
pub use ops::{close_mr_impl, create_mr_impl, update_mr_impl};
