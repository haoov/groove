//! The row-based marks and words the hunks replaced, kept as what the hunks must agree with.

use std::collections::BTreeMap;
use std::ops::Range;

use groove_text::Document;
use groove_types::{LineMark, Row, RowKind, word_diff_pairs};

use crate::words::between;

/// The columns a row draws that the row it pairs with does not, by row.
pub(super) type Words = BTreeMap<usize, Vec<Range<usize>>>;

/// What each row of a pair says that the other does not, in the columns it draws.
pub(super) fn words(rows: &[Row], old: &Document, new: &Document) -> Words {
    let mut words = Words::new();
    for (gone, came) in word_diff_pairs(rows) {
        let (Some(before), Some(after)) = (line(old, rows[gone].old), line(new, rows[came].new))
        else {
            continue;
        };
        let (left, right) = between(&before, &after);
        keep(&mut words, gone, left);
        keep(&mut words, came, right);
    }
    words
}

/// The same, by the line of the new file a row shows.
pub(super) fn by_line(rows: &[Row], words: &Words) -> BTreeMap<u32, Vec<Range<usize>>> {
    words
        .iter()
        .filter_map(|(at, ranges)| Some((rows.get(*at)?.new?, ranges.clone())))
        .collect()
}

fn keep(words: &mut Words, at: usize, ranges: Vec<Range<usize>>) {
    if !ranges.is_empty() {
        words.insert(at, ranges);
    }
}

fn line(document: &Document, at: Option<u32>) -> Option<String> {
    Some(document.line(at? as usize)?.to_string())
}

/// What each line of the new file did; a removal with nothing in its place marks the next line.
pub(super) fn marks(rows: &[Row]) -> BTreeMap<u32, LineMark> {
    let mut marks = BTreeMap::new();
    let mut at = 0;
    while at < rows.len() {
        let gone = run_of(rows, at, RowKind::Removed);
        let came = run_of(rows, at + gone, RowKind::Added);
        if gone == 0 && came == 0 {
            at += 1;
            continue;
        }
        let mark = match (gone, came) {
            (0, _) => LineMark::Added,
            (_, 0) => LineMark::Removed,
            _ => LineMark::Changed,
        };
        marks.extend(marked(rows, at + gone, came, mark));
        at += gone + came;
    }
    marks
}

/// The lines a change marks: the ones that came, or the line a removal closed on.
fn marked(rows: &[Row], at: usize, came: usize, mark: LineMark) -> Vec<(u32, LineMark)> {
    if came == 0 {
        let next = rows[at..].iter().find_map(|row| row.new);
        return next.map(|line| (line, mark)).into_iter().collect();
    }
    rows[at..at + came]
        .iter()
        .filter_map(|row| row.new)
        .map(|line| (line, mark))
        .collect()
}

fn run_of(rows: &[Row], from: usize, kind: RowKind) -> usize {
    rows[from.min(rows.len())..]
        .iter()
        .take_while(|row| row.kind == kind)
        .count()
}
