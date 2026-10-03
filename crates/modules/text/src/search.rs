//! Finding text in one document.

use std::ops::Range;

/// Where a query was found: the line, and the characters inside that line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub line: usize,
    pub range: Range<usize>,
}

/// Every occurrence of `query` in `line`, ignoring case.
pub(crate) fn found(line: usize, text: &str, query: &str) -> Vec<Found> {
    groove_types::occurrences(text, query)
        .into_iter()
        .map(|range| Found { line, range })
        .collect()
}
