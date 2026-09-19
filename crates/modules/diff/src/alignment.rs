//! How two files line up, and what each line and each word of a pair did.

use std::collections::BTreeMap;
use std::ops::Range;

use groove_text::Document;
use groove_types::{LineMark, Row, RowKind, word_diff_pairs};
use imara_diff::{Algorithm, Diff, InternedInput, Interner, Token};

use crate::words::between;

/// The columns a row draws that the row it pairs with does not, by row.
pub type Words = BTreeMap<usize, Vec<Range<usize>>>;

/// Unchanged lines kept either side of a change.
pub const CONTEXT: u32 = 3;

/// Lines a gap must hide to be worth a row of its own.
const GAP_MIN: u32 = 2;

/// The rows `align` builds.
trait Rows {
    fn context(old: u32, new: u32) -> Self;
    fn removed(old: u32) -> Self;
    fn added(new: u32) -> Self;
    fn gap(lines: u32) -> Self;
}

impl Rows for Row {
    fn context(old: u32, new: u32) -> Self {
        Self {
            old: Some(old),
            new: Some(new),
            kind: RowKind::Context,
        }
    }

    fn removed(old: u32) -> Self {
        Self {
            old: Some(old),
            new: None,
            kind: RowKind::Removed,
        }
    }

    fn added(new: u32) -> Self {
        Self {
            old: None,
            new: Some(new),
            kind: RowKind::Added,
        }
    }

    fn gap(lines: u32) -> Self {
        Self {
            old: None,
            new: None,
            kind: RowKind::Gap(lines),
        }
    }
}

/// How the two files line up, as the rows a reader sees.
pub fn align(old: &Document, new: &Document, context: u32) -> Vec<Row> {
    let input = interned(old, new);
    let diff = Diff::compute(Algorithm::Histogram, &input);
    let mut rows = Vec::new();
    let mut at = (0, 0);
    for hunk in diff.hunks() {
        lead(
            &mut rows,
            at,
            (hunk.before.start, hunk.after.start),
            context,
        );
        rows.extend(hunk.before.clone().map(Row::removed));
        rows.extend(hunk.after.clone().map(Row::added));
        at = (hunk.before.end, hunk.after.end);
    }
    let ends = (lines(old), lines(new));
    trail(&mut rows, at, ends, context);
    rows
}

/// The unchanged lines between where the last hunk ended and where this one starts:
/// context on each side, and a gap for what neither side needs.
fn lead(rows: &mut Vec<Row>, from: (u32, u32), to: (u32, u32), context: u32) {
    let unchanged = to.0.saturating_sub(from.0);
    let head = unchanged.min(context);
    let tail = unchanged.saturating_sub(head).min(context);
    let skipped = unchanged - head - tail;
    if skipped < GAP_MIN {
        return rows.extend(run(from, unchanged));
    }
    rows.extend(run(from, head));
    rows.push(Row::gap(skipped));
    rows.extend(run((to.0 - tail, to.1 - tail), tail));
}

/// The context after the last hunk, and the gap to the end of the file.
fn trail(rows: &mut Vec<Row>, from: (u32, u32), ends: (u32, u32), context: u32) {
    let left = ends.0.saturating_sub(from.0);
    let shown = left.min(context);
    let skipped = left - shown;
    if skipped < GAP_MIN {
        return rows.extend(run(from, left));
    }
    rows.extend(run(from, shown));
    rows.push(Row::gap(skipped));
}

fn run(from: (u32, u32), lines: u32) -> impl Iterator<Item = Row> {
    (0..lines).map(move |at| Row::context(from.0 + at, from.1 + at))
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

/// What each row of a pair says that the other does not, in the columns it draws.
pub fn words(rows: &[Row], old: &Document, new: &Document) -> Words {
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
pub fn by_line(rows: &[Row], words: &Words) -> BTreeMap<u32, Vec<Range<usize>>> {
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

/// What each line of the new file did. A removal with nothing in its place marks the
/// line that closed the gap.
pub fn marks(rows: &[Row]) -> BTreeMap<u32, LineMark> {
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
