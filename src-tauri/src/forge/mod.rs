//! The forge feature: MRs/PRs on GitLab and GitHub, over their APIs directly.
//! The CLIs (`glab`, `gh`) are consulted only for tokens — see `auth`.

mod api;
mod auth;
mod client;
mod commands;
mod github;
mod gitlab;
mod ops;
mod queue;

// Glob re-export is required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function, so they must be re-exported from this module too.
pub use commands::*;
pub use ops::{close_mr_impl, create_mr_impl, update_mr_impl};
pub use queue::*;
