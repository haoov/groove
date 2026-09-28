//! Hunks moved along an edit still line the two sides up: nothing lost, nothing paired wrong.

use groove_text::{Document, Touched};
use groove_types::RowKind;

use crate::alignment::rows_of;
use crate::hunks;
use crate::moved::moved;

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

const LINES: [&str; 5] = ["let one = 1;", "fn two() {}", "}", "", "three(4);"];

fn lines(seed: &mut Seeded, count: usize) -> Vec<String> {
    (0..count)
        .map(|_| LINES[seed.below(LINES.len())].to_string())
        .collect()
}

fn text(lines: &[String]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// One edit of `new`, as the lines it replaced and what now stands there.
fn edit(seed: &mut Seeded, new: &mut Vec<String>) -> Touched {
    let line = seed.below(new.len());
    match seed.below(4) {
        0 => {
            let count = 1 + seed.below(3);
            let came = lines(seed, count);
            let count = came.len() as u32;
            let kept = new[line].clone();
            new.splice(line..=line, came.into_iter().chain([kept]));
            touched(line, 1, count + 1)
        }
        1 if new.len() > 2 => {
            let gone = 1 + seed.below((new.len() - line).min(3));
            new.splice(line..line + gone, [String::new()]);
            touched(line, gone as u32, 1)
        }
        2 if line + 1 < new.len() => {
            let joined = format!("{}{}", new[line], new[line + 1]);
            new.splice(line..line + 2, [joined]);
            touched(line, 2, 1)
        }
        _ => {
            new[line].push_str(" // typed");
            touched(line, 1, 1)
        }
    }
}

fn touched(line: usize, gone: u32, came: u32) -> Touched {
    Touched {
        line: line as u32,
        gone,
        came,
    }
}

#[test]
fn moved_hunks_still_line_the_sides_up() {
    let mut seed = Seeded(3);
    for _ in 0..500 {
        let count = 3 + seed.below(30);
        let old = lines(&mut seed, count);
        let mut new = old.clone();
        for _ in 0..seed.below(4) {
            let at = seed.below(new.len());
            new[at] = "changed".to_string();
        }
        let (left, right) = (
            Document::plain("a", &text(&old)),
            Document::plain("a", &text(&new)),
        );
        let mut found = hunks(&left, &right);
        for _ in 0..1 + seed.below(5) {
            let one = edit(&mut seed, &mut new);
            found = moved(&found, one).0;
        }
        let right = Document::plain("a", &text(&new));
        let ends = (left.lines() as u32, right.lines() as u32);
        let rows = rows_of(&found, ends, u32::MAX, &[]);
        let olds: Vec<u32> = rows.iter().filter_map(|row| row.old).collect();
        let news: Vec<u32> = rows.iter().filter_map(|row| row.new).collect();
        assert_eq!(olds, (0..ends.0).collect::<Vec<_>>(), "{found:?}");
        assert_eq!(news, (0..ends.1).collect::<Vec<_>>(), "{found:?}");
        for row in rows.iter().filter(|row| row.kind == RowKind::Context) {
            let (a, b) = (row.old.unwrap(), row.new.unwrap());
            assert_eq!(left.line(a as usize), right.line(b as usize), "{found:?}");
        }
    }
}
