//! The diff: what changed in a worktree, and how its two sides line up. The summary
//! is every changed file with its counts, cheap enough to draw a list from.

mod alignment;
mod changes;
mod commit;
mod opened;
mod summary;
mod words;

#[cfg(test)]
mod tests;

pub use alignment::{Words, align, by_line, marks};
pub use changes::{Aligned, At, Changes, aligned, changes};
pub use commit::{at_commit, commits, opened_at};
pub use groove_text::{Buffer, Document};

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

/// The column a character of the line is drawn at.
pub fn display_at(line: &str, column: usize, width: usize) -> usize {
    groove_text::display_of(line, column, width)
}

/// The character a drawn column belongs to.
pub fn columns(line: &str, display: usize, width: usize) -> usize {
    groove_text::column_of(line, display, width)
}
pub use opened::{Derived, Opened, derived, from_documents, from_text, opened, reopened};
pub use summary::{MAX_BYTES, summary, summary_against};
