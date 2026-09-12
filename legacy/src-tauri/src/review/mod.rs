//! Reading changes: diffs, blame, commit history, and the parsers under them.

mod blame;
mod commits;
mod diff;
mod parse;
pub mod types;

// Keep the glob re-exports: `generate_handler!` resolves `__cmd__*` symbols at this path.
pub use blame::*;
pub use commits::*;
pub use diff::*;
