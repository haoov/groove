//! A file's change as hunks: the rows they stand for, each new line's mark, each pair's words.

use std::collections::BTreeMap;
use std::ops::Range;

use groove_text::{Document, Touched};
use groove_types::{Hunk, LineMark};

use crate::alignment::{CONTEXT, hunks};
use crate::layout::Layout;
use crate::moved::moved;
use crate::words::between;

/// Columns of a line, by the line.
pub type Lined = BTreeMap<u32, Vec<Range<usize>>>;

/// The columns each side's lines changed, by side.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sided {
    pub old: Lined,
    pub new: Lined,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hunked {
    pub hunks: Vec<Hunk>,
    pub layout: Layout,
    pub marks: BTreeMap<u32, LineMark>,
    pub words: Sided,
}

impl Hunked {
    /// The two sides diffed whole; `opened` names the old lines the gaps gave up.
    pub fn of(old: &Document, new: &Document, opened: &[Range<u32>]) -> Self {
        let found = hunks(old, new);
        let ends = ends(old, new);
        Self {
            layout: Layout::of(&found, ends, CONTEXT, opened),
            marks: marks_of(&found, ends.1),
            words: words_of(&found, old, new),
            hunks: found,
        }
    }

    /// One edit of the new side, `new` being its text after it, with nothing diffed again.
    pub fn edited(&mut self, edit: Touched, old: &Document, new: &Document, opened: &[Range<u32>]) {
        let (mut found, at) = moved(&self.hunks, edit);
        let ends = ends(old, new);
        let merged = clamped(&found[at], ends);
        let grown = i64::from(edit.came) - i64::from(edit.gone);
        self.words.moved(&merged, grown);
        let merged = trimmed(merged, old, new);
        match merged.before.is_empty() && merged.after.is_empty() {
            true => drop(found.remove(at)),
            false => {
                let words = words_of(std::slice::from_ref(&merged), old, new);
                self.words.old.extend(words.old);
                self.words.new.extend(words.new);
                found[at] = merged;
            }
        }
        self.layout = Layout::of(&found, ends, CONTEXT, opened);
        self.marks = marks_of(&found, ends.1);
        self.hunks = found;
    }
}

impl Sided {
    /// The words of the lines `merged` now covers let go, and the new lines under it moved.
    fn moved(&mut self, merged: &Hunk, grown: i64) {
        self.old.retain(|line, _| !merged.before.contains(line));
        let was = u32::try_from(i64::from(merged.after.end) - grown).unwrap_or(0);
        let kept = std::mem::take(&mut self.new);
        self.new = kept
            .into_iter()
            .filter(|(line, _)| *line < merged.after.start || *line >= was)
            .map(|(line, ranges)| match line >= was {
                true => (u32::try_from(i64::from(line) + grown).unwrap_or(0), ranges),
                false => (line, ranges),
            })
            .collect();
    }
}

fn ends(old: &Document, new: &Document) -> (u32, u32) {
    (old.lines() as u32, new.lines() as u32)
}

/// The hunk held inside both sides' lines.
fn clamped(hunk: &Hunk, ends: (u32, u32)) -> Hunk {
    let within = |range: &Range<u32>, end: u32| range.start.min(end)..range.end.min(end);
    Hunk {
        before: within(&hunk.before, ends.0),
        after: within(&hunk.after, ends.1),
    }
}

/// The hunk without the lines at either end that both sides still read the same.
fn trimmed(mut hunk: Hunk, old: &Document, new: &Document) -> Hunk {
    let same = |gone: u32, came: u32| old.line(gone as usize) == new.line(came as usize);
    while !hunk.before.is_empty()
        && !hunk.after.is_empty()
        && same(hunk.before.start, hunk.after.start)
    {
        hunk.before.start += 1;
        hunk.after.start += 1;
    }
    while !hunk.before.is_empty()
        && !hunk.after.is_empty()
        && same(hunk.before.end - 1, hunk.after.end - 1)
    {
        hunk.before.end -= 1;
        hunk.after.end -= 1;
    }
    hunk
}

/// What each line of the new file did; a removal with nothing in its place marks the next line.
pub fn marks_of(hunks: &[Hunk], new_lines: u32) -> BTreeMap<u32, LineMark> {
    let mut marks = BTreeMap::new();
    for hunk in hunks {
        let mark = match (hunk.before.is_empty(), hunk.after.is_empty()) {
            (true, true) => continue,
            (true, false) => LineMark::Added,
            (false, true) => LineMark::Removed,
            (false, false) => LineMark::Changed,
        };
        let lines = match hunk.after.is_empty() {
            true => hunk.after.start..(hunk.after.start + 1).min(new_lines),
            false => hunk.after.clone(),
        };
        marks.extend(lines.map(|line| (line, mark)));
    }
    marks
}

/// The words each pair changed; a hunk pairs its lines one to one when both sides hold as many.
pub fn words_of(hunks: &[Hunk], old: &Document, new: &Document) -> Sided {
    let mut words = Sided::default();
    for hunk in hunks.iter().filter(|hunk| paired(hunk)) {
        for (gone, came) in hunk.before.clone().zip(hunk.after.clone()) {
            let (Some(before), Some(after)) = (old.line(gone as usize), new.line(came as usize))
            else {
                continue;
            };
            let (left, right) = between(&before, &after);
            keep(&mut words.old, gone, left);
            keep(&mut words.new, came, right);
        }
    }
    words
}

fn paired(hunk: &Hunk) -> bool {
    !hunk.before.is_empty() && hunk.before.len() == hunk.after.len()
}

fn keep(words: &mut Lined, line: u32, ranges: Vec<Range<usize>>) {
    if !ranges.is_empty() {
        words.insert(line, ranges);
    }
}
