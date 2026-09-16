use groove_text::Document;
use groove_types::{Row, RowKind};
use imara_diff::{Algorithm, Diff, InternedInput, Interner, Token};

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
