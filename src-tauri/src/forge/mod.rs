//! The forge feature: MRs/PRs on GitLab and GitHub. Transport and tokens live
//! in `core::forge`.

mod client;
mod commands;
mod github;
mod gitlab;
mod ops;
mod queue;

// Keep the glob re-exports: `generate_handler!` resolves `__cmd__*` symbols at this path.
pub use commands::*;
pub use ops::{close_mr_impl, create_mr_impl, mr_target_for, update_mr_impl};
pub use queue::*;
