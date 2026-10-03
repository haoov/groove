//! How two files line up: where they differ, and the rows that stand for it.

use groove_text::Document;
use groove_types::Hunk;
use imara_diff::{Algorithm, Diff, InternedInput, Interner, Token};

/// Unchanged lines kept either side of a change.
pub const CONTEXT: u32 = 3;

#[cfg(test)]
pub(crate) use rowed::{align, rows_of};

/// Where the two files differ, in order.
pub fn hunks(old: &Document, new: &Document) -> Vec<Hunk> {
    let input = interned(old, new);
    let diff = Diff::compute(Algorithm::Histogram, &input);
    diff.hunks()
        .map(|one| Hunk {
            before: one.before,
            after: one.after,
        })
        .collect()
}

fn interned<'a>(old: &'a Document, new: &'a Document) -> InternedInput<Line<'a>> {
    let mut interner = Interner::new(lines(old) as usize + lines(new) as usize);
    let before = tokens(old, &mut interner);
    let after = tokens(new, &mut interner);
    InternedInput {
        before,
        after,
        interner,
    }
}

type Line<'a> = std::borrow::Cow<'a, str>;

fn tokens<'a>(document: &'a Document, interner: &mut Interner<Line<'a>>) -> Vec<Token> {
    (0..lines(document))
        .filter_map(|at| document.line(at as usize))
        .map(|line| interner.intern(line))
        .collect()
}

fn lines(document: &Document) -> u32 {
    document.lines() as u32
}

/// The two files as rows, which the tests hold the hunks' layout against.
#[cfg(test)]
mod rowed {
    use std::ops::Range;

    use groove_text::Document;
    use groove_types::{Hunk, Row};

    use crate::layout::Layout;

    /// How the two files line up, as rows; `opened` names the old-side lines a gap gives up.
    pub fn align(old: &Document, new: &Document, context: u32, opened: &[Range<u32>]) -> Vec<Row> {
        let ends = (super::lines(old), super::lines(new));
        rows_of(&super::hunks(old, new), ends, context, opened)
    }

    /// The rows the hunks stand for, between sides `ends` lines long.
    pub fn rows_of(
        hunks: &[Hunk],
        ends: (u32, u32),
        context: u32,
        opened: &[Range<u32>],
    ) -> Vec<Row> {
        Layout::of(hunks, ends, context, opened).all().collect()
    }
}
