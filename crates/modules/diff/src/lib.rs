//! The diff: what changed in a worktree, and how its two sides line up.

mod alignment;
mod changes;
mod commit;
mod hunked;
mod layout;
mod moved;
mod opened;
mod summary;
mod words;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use alignment::{align, hunks};
pub use changes::{Aligned, At, Changes, aligned, changes};
pub use commit::{at_commit, commits, opened_at};
pub use groove_text::{Buffer, Document, column_of, display_of};
pub use layout::Layout;

/// A line as the surface draws it, and its colours over it.
pub fn shown(
    line: &str,
    spans: &[groove_types::Highlight],
    width: usize,
) -> (String, Vec<groove_types::Highlight>) {
    (
        groove_text::expand(line, width),
        groove_text::spans_of(spans, line, width),
    )
}

pub use opened::{Derived, Opened, derived, from_documents, from_text, opened, reopened};
pub use summary::{summary, summary_against};
