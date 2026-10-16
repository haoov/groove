//! A fuzzy match with a score: runs and word starts count, gaps cost.

/// Where a query's characters landed in a text, and how well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sought {
    pub score: i32,
    /// Character indices of the text, one per character of the query.
    pub at: Vec<usize>,
}

const HIT: i32 = 16;
const RUN: i32 = 24;
const START: i32 = 12;
const GAP: i32 = 1;

/// One cell: the best score with this character of the query here, and where the one before is.
type Cell = Option<(i32, usize)>;

/// The best placing of `query`'s characters in `hay`, in order, case aside; spaces skipped.
pub fn sought(hay: &str, query: &str) -> Option<Sought> {
    if !query.contains(char::is_whitespace) && !crate::fuzzy(hay, query) {
        return None;
    }
    let fold = |c: char| c.to_lowercase().next().unwrap_or(c);
    let hay: Vec<char> = hay.chars().collect();
    let low: Vec<char> = hay.iter().map(|c| fold(*c)).collect();
    let want: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if want.is_empty() {
        return Some(Sought {
            score: 0,
            at: Vec::new(),
        });
    }
    let mut rest = low.iter();
    if !want.iter().all(|wanted| rest.any(|one| one == wanted)) {
        return None;
    }
    let mut best: Vec<Vec<Cell>> = Vec::with_capacity(want.len());
    for (i, wanted) in want.iter().enumerate() {
        let row = placed((&hay, &low), *wanted, best.last().filter(|_| i > 0));
        best.push(row);
    }
    traced(&best)
}

/// Every place one character of the query can stand, given where the one before could.
fn placed((hay, low): (&[char], &[char]), wanted: char, before: Option<&Vec<Cell>>) -> Vec<Cell> {
    let mut row = vec![None; hay.len()];
    let mut carried: Cell = None;
    for j in 0..hay.len() {
        if let (Some(before), true) = (before, j > 0) {
            let last = before[j - 1].map(|(score, _)| (score, j - 1));
            let kept = carried.map(|(score, from)| (score - GAP, from));
            carried = better(kept, last);
        }
        if low[j] != wanted {
            continue;
        }
        let bonus = HIT
            + if j == 0 || !hay[j - 1].is_alphanumeric() {
                START
            } else {
                0
            };
        row[j] = match before {
            None => Some((bonus - j as i32 * GAP / 4, usize::MAX)),
            Some(before) => {
                let run = j
                    .checked_sub(1)
                    .and_then(|k| before[k])
                    .map(|(s, _)| (s + bonus + RUN, j - 1));
                let gap = carried.map(|(score, from)| (score + bonus, from));
                better(run, gap)
            }
        };
    }
    row
}

fn better(a: Cell, b: Cell) -> Cell {
    match (a, b) {
        (Some(x), Some(y)) => Some(if x.0 >= y.0 { x } else { y }),
        (x, y) => x.or(y),
    }
}

/// The best last cell, walked back to where each character stands.
fn traced(best: &[Vec<Cell>]) -> Option<Sought> {
    let last = best.last()?;
    let (end, score) = last
        .iter()
        .enumerate()
        .filter_map(|(j, one)| one.map(|(score, _)| (j, score)))
        .max_by_key(|(j, score)| (*score, std::cmp::Reverse(*j)))?;
    let mut at = vec![end];
    for row in best[1..].iter().rev() {
        let (_, from) = row[at[0]]?;
        at.insert(0, from);
    }
    Some(Sought { score, at })
}

/// The rows `query` finds, the best first; an empty query keeps them all, in order.
pub fn ranked<T>(
    rows: impl IntoIterator<Item = T>,
    text: impl Fn(&T) -> String,
    query: &str,
) -> Vec<T> {
    if query.trim().is_empty() {
        return rows.into_iter().collect();
    }
    let mut scored: Vec<(i32, T)> = rows
        .into_iter()
        .filter_map(|row| Some((sought(&text(&row), query)?.score, row)))
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(_, row)| row).collect()
}

/// Where each word lands in `text`, as runs of characters; nothing when a word misses.
pub fn spans(text: &str, words: &[&str]) -> Vec<std::ops::Range<usize>> {
    let mut at: Vec<usize> = Vec::new();
    for word in words {
        let Some(found) = sought(text, word) else {
            return Vec::new();
        };
        at.extend(found.at);
    }
    runs(at)
}

/// Character positions as runs, each position once.
pub fn runs(mut at: Vec<usize>) -> Vec<std::ops::Range<usize>> {
    at.sort_unstable();
    at.dedup();
    let mut out: Vec<std::ops::Range<usize>> = Vec::new();
    for one in at {
        match out.last_mut() {
            Some(last) if last.end == one => last.end += 1,
            _ => out.push(one..one + 1),
        }
    }
    out
}
