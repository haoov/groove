//! Finding text in one document.

use std::ops::Range;

/// Where a query was found: the line, and the bytes inside that line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub line: usize,
    pub range: Range<usize>,
}

/// Every occurrence of `query` in `line`, ignoring case.
pub(crate) fn found(line: usize, text: &str, query: &str) -> Vec<Found> {
    if query.is_empty() {
        return Vec::new();
    }
    let (haystack, needle) = (text.to_lowercase(), query.to_lowercase());
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(hit) = haystack[at..].find(&needle) {
        let start = at + hit;
        out.push(Found {
            line,
            range: start..start + needle.len(),
        });
        at = start + needle.len();
    }
    out
}
