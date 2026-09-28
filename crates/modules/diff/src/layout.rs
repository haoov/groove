//! The rows a file's hunks stand for, held as runs: an entry a run, not a row a line.

use std::ops::Range;

use groove_types::{Hunk, Row, RowKind};

/// Lines a gap must hide to be worth a row of its own.
const GAP_MIN: u32 = 2;

/// What a run of rows shows, from its first line on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    Same {
        old: u32,
        new: u32,
    },
    Gone {
        old: u32,
    },
    Came {
        new: u32,
    },
    /// Lines neither side shows, drawn as one row.
    Gap {
        lines: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placed {
    first: usize,
    rows: u32,
    run: Run,
}

/// Every row of one file, found by its index without being stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    runs: Vec<Placed>,
    rows: usize,
}

impl Layout {
    /// The rows of `hunks` between sides `ends` lines long; `opened` names the lines gaps gave up.
    pub fn of(hunks: &[Hunk], ends: (u32, u32), context: u32, opened: &[Range<u32>]) -> Self {
        let mut out = Self::default();
        let mut at = (0, 0);
        for hunk in hunks {
            let to = (hunk.before.start, hunk.after.start);
            out.lead(at, to, context, opened);
            out.push(
                hunk.before.len() as u32,
                Run::Gone {
                    old: hunk.before.start,
                },
            );
            out.push(
                hunk.after.len() as u32,
                Run::Came {
                    new: hunk.after.start,
                },
            );
            at = (hunk.before.end, hunk.after.end);
        }
        out.trail(at, ends, context, opened);
        out
    }

    /// One tall row, for a file too long to align.
    pub fn long() -> Self {
        let mut out = Self::default();
        out.push(1, Run::Gap { lines: 0 });
        out
    }

    pub fn len(&self) -> usize {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    pub fn row(&self, at: usize) -> Option<Row> {
        let placed = self.run_at(at)?;
        let off = (at - placed.first) as u32;
        Some(match placed.run {
            Run::Same { old, new } => row(Some(old + off), Some(new + off), RowKind::Context),
            Run::Gone { old } => row(Some(old + off), None, RowKind::Removed),
            Run::Came { new } => row(None, Some(new + off), RowKind::Added),
            Run::Gap { lines } => row(None, None, RowKind::Gap(lines)),
        })
    }

    /// The rows from `at`, in order.
    pub fn rows(&self, at: Range<usize>) -> impl Iterator<Item = Row> + '_ {
        at.filter_map(|one| self.row(one))
    }

    pub fn all(&self) -> impl Iterator<Item = Row> + '_ {
        self.rows(0..self.rows)
    }

    /// Each run as its first row, how many rows it stands, and the kind of all of them.
    pub fn spans(&self) -> impl Iterator<Item = (usize, usize, RowKind)> + '_ {
        self.runs.iter().map(|placed| {
            let kind = match placed.run {
                Run::Same { .. } => RowKind::Context,
                Run::Gone { .. } => RowKind::Removed,
                Run::Came { .. } => RowKind::Added,
                Run::Gap { lines } => RowKind::Gap(lines),
            };
            (placed.first, placed.rows as usize, kind)
        })
    }

    /// The row a new-side line stands on, when a row shows it.
    pub fn position(&self, line: u32) -> Option<usize> {
        self.runs.iter().find_map(|placed| {
            let first = match placed.run {
                Run::Same { new, .. } | Run::Came { new } => new,
                _ => return None,
            };
            let off = line.checked_sub(first).filter(|off| *off < placed.rows)?;
            Some(placed.first + off as usize)
        })
    }

    /// The highest line number any row shows, on either side.
    pub fn highest(&self) -> u32 {
        let last = |placed: &Placed| match placed.run {
            Run::Same { old, new } => old.max(new) + placed.rows - 1,
            Run::Gone { old } => old + placed.rows - 1,
            Run::Came { new } => new + placed.rows - 1,
            Run::Gap { .. } => 0,
        };
        self.runs.iter().map(last).max().unwrap_or(0)
    }

    fn run_at(&self, at: usize) -> Option<&Placed> {
        if at >= self.rows {
            return None;
        }
        let found = self.runs.partition_point(|placed| placed.first <= at);
        self.runs.get(found.checked_sub(1)?)
    }

    fn push(&mut self, rows: u32, run: Run) {
        if rows == 0 {
            return;
        }
        self.runs.push(Placed {
            first: self.rows,
            rows,
            run,
        });
        self.rows += rows as usize;
    }

    fn same(&mut self, from: (u32, u32), lines: u32) {
        let run = Run::Same {
            old: from.0,
            new: from.1,
        };
        self.push(lines, run);
    }

    /// The unchanged lines between two hunks: context on each side, a gap between.
    fn lead(&mut self, from: (u32, u32), to: (u32, u32), context: u32, opened: &[Range<u32>]) {
        let unchanged = to.0.saturating_sub(from.0);
        let head = unchanged.min(context);
        let tail = unchanged.saturating_sub(head).min(context);
        let skipped = unchanged - head - tail;
        if skipped < GAP_MIN {
            return self.same(from, unchanged);
        }
        self.same(from, head);
        self.hidden((from.0 + head, from.1 + head), skipped, opened);
        self.same((to.0 - tail, to.1 - tail), tail);
    }

    /// The context after the last hunk, and the gap to the end of the file.
    fn trail(&mut self, from: (u32, u32), ends: (u32, u32), context: u32, opened: &[Range<u32>]) {
        let left = ends.0.saturating_sub(from.0);
        let shown = left.min(context);
        let skipped = left - shown;
        if skipped < GAP_MIN {
            return self.same(from, left);
        }
        self.same(from, shown);
        self.hidden((from.0 + shown, from.1 + shown), skipped, opened);
    }

    /// The lines a gap covers: the opened ones as rows, the rest as gaps of their own.
    fn hidden(&mut self, from: (u32, u32), lines: u32, opened: &[Range<u32>]) {
        let mut at = 0;
        for span in opened {
            let start = span.start.max(from.0).min(from.0 + lines);
            let end = span.end.max(from.0).min(from.0 + lines);
            if start >= end {
                continue;
            }
            let before = start - from.0 - at;
            self.push(u32::from(before > 0), Run::Gap { lines: before });
            self.same((start, from.1 + (start - from.0)), end - start);
            at = end - from.0;
        }
        let left = lines - at;
        self.push(u32::from(left > 0), Run::Gap { lines: left });
    }
}

fn row(old: Option<u32>, new: Option<u32>, kind: RowKind) -> Row {
    Row { old, new, kind }
}
