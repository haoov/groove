//! The diff: what changed in a worktree, and how its two sides line up. The summary
//! is every changed file with its counts, cheap enough to draw a list from.

mod summary;

#[cfg(test)]
mod tests;

pub use summary::{MAX_BYTES, summary};
