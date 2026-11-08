//! A change a write would make, held whole while it waits: both sides parsed, and how they line up.

use groove_diff::{Aligned, Document, from_sides};

#[derive(Debug)]
pub struct Review {
    pub aligned: Aligned,
    pub old: Document,
    pub new: Document,
}

/// `before` and `after` at `path`, whose extension names their grammar.
pub fn review(path: &str, before: &str, after: &str) -> Review {
    let (old, new) = (Document::new(path, before), Document::new(path, after));
    Review {
        aligned: from_sides(path, &old, &new, &[]),
        old,
        new,
    }
}
