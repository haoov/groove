//! The diff: what changed in a worktree, and how its two sides line up. The summary
//! is every changed file with its counts, cheap enough to draw a list from.

mod alignment;
mod opened;
mod summary;

#[cfg(test)]
mod tests;

pub use alignment::{CONTEXT, align, marks};
pub use groove_text::Document;
pub use opened::{MAX_SHOWN_BYTES, Opened, from_text, opened, reopened};
pub use summary::{MAX_BYTES, summary};
