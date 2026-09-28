//! Hunks stand for the same rows, marks and words the rows themselves give.

use groove_text::Document;

use super::reference::{by_line, marks, words};
use crate::alignment::{CONTEXT, rows_of};
use crate::hunked::{marks_of, words_of};
use crate::hunks;
use crate::layout::Layout;

/// A small generator the runs repeat exactly.
struct Seeded(u64);

impl Seeded {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n.max(1)
    }
}

const WORDS: [&str; 6] = [
    "let one = 1;",
    "fn two() {}",
    "}",
    "",
    "    three(4);",
    "// five",
];

fn text(lines: &[&str]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// A file, and the same file after a few lines were added, taken out or rewritten.
fn pair(seed: &mut Seeded) -> (String, String) {
    let count = 5 + seed.below(40);
    let old: Vec<&str> = (0..count).map(|_| WORDS[seed.below(WORDS.len())]).collect();
    let mut new = old.clone();
    for _ in 0..1 + seed.below(6) {
        let at = seed.below(new.len() + 1);
        match seed.below(3) {
            0 => new.insert(at, WORDS[seed.below(WORDS.len())]),
            1 if at < new.len() => drop(new.remove(at)),
            _ if at < new.len() => new[at] = "let one = 11;",
            _ => {}
        }
    }
    (text(&old), text(&new))
}

fn sides(old: &str, new: &str) -> (Document, Document) {
    (Document::plain("a.rs", old), Document::plain("a.rs", new))
}

#[test]
fn the_layout_finds_every_row_the_rows_hold() {
    let mut seed = Seeded(7);
    for _ in 0..300 {
        let (before, after) = pair(&mut seed);
        let (old, new) = sides(&before, &after);
        let found = hunks(&old, &new);
        let ends = (old.lines() as u32, new.lines() as u32);
        let layout = Layout::of(&found, ends, CONTEXT, &[]);
        let rows = rows_of(&found, ends, CONTEXT, &[]);
        assert_eq!(layout.len(), rows.len());
        for (at, row) in rows.iter().enumerate() {
            assert_eq!(layout.row(at).as_ref(), Some(row), "row {at} of {after:?}");
            if let Some(line) = row.new {
                assert_eq!(layout.position(line), Some(at), "line {line} of {after:?}");
            }
        }
    }
}

#[test]
fn hunks_mark_the_lines_and_the_words_the_rows_mark() {
    let mut seed = Seeded(11);
    for _ in 0..300 {
        let (before, after) = pair(&mut seed);
        let (old, new) = sides(&before, &after);
        let found = hunks(&old, &new);
        let ends = (old.lines() as u32, new.lines() as u32);
        let rows = rows_of(&found, ends, CONTEXT, &[]);
        assert_eq!(
            marks_of(&found, ends.1),
            marks(&rows),
            "{before:?} to {after:?}"
        );
        let by_rows = words(&rows, &old, &new);
        let sided = words_of(&found, &old, &new);
        assert_eq!(
            sided.new,
            by_line(&rows, &by_rows),
            "{before:?} to {after:?}"
        );
        let olds: crate::hunked::Lined = by_rows
            .iter()
            .filter_map(|(at, ranges)| {
                Some((
                    rows[*at].old.filter(|_| rows[*at].new.is_none())?,
                    ranges.clone(),
                ))
            })
            .collect();
        assert_eq!(sided.old, olds, "{before:?} to {after:?}");
    }
}
