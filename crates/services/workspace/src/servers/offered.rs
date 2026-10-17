//! What the servers offer to type at the caret, and the word an offer replaces.

use groove_types::{Caret, Completion};

use super::Servers;

/// One completion asked at a caret of a file, its items once answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offered {
    pub path: String,
    pub at: Caret,
    /// Which ask it is; an answer to an earlier one is dropped.
    pub asked: u64,
    pub items: Vec<Completion>,
}

impl Servers {
    /// A completion asked at `at`; the items of the last ask on the file stand until it answers.
    pub fn offering(&mut self, path: &str, at: Caret) -> u64 {
        self.asks += 1;
        let before = self.offered.take().filter(|one| one.path == path);
        self.offered = Some(Offered {
            path: path.to_string(),
            at,
            asked: self.asks,
            items: before.map(|one| one.items).unwrap_or_default(),
        });
        self.asks
    }

    /// The answer to ask `asked`, kept while no later ask or dismissal came.
    pub fn offers(&mut self, asked: u64, items: Vec<Completion>) {
        if let Some(one) = self.offered.as_mut().filter(|one| one.asked == asked) {
            one.items = items;
        }
    }

    pub fn dismiss(&mut self) {
        self.offered = None;
    }
}

/// The items whose filter holds `typed` in order, case aside: those it begins first, then by sort.
pub fn matching<'a>(items: &'a [Completion], typed: &str) -> Vec<&'a Completion> {
    let typed = typed.to_lowercase();
    let mut kept: Vec<(bool, &Completion)> = items
        .iter()
        .filter_map(|one| {
            let filter = one.filter.to_lowercase();
            let mut left = filter.chars();
            let holds = typed.chars().all(|c| left.any(|f| f == c));
            holds.then(|| (!filter.starts_with(&typed), one))
        })
        .collect();
    kept.sort_by(|a, b| (a.0, &a.1.sort, &a.1.label).cmp(&(b.0, &b.1.sort, &b.1.label)));
    kept.into_iter().map(|(_, one)| one).collect()
}

/// The column the word ending at `column` of `line` starts at.
pub fn word_start(line: &str, column: usize) -> usize {
    let before: Vec<char> = line.chars().take(column).collect();
    let word = before
        .iter()
        .rev()
        .take_while(|c| c.is_alphanumeric() || **c == '_')
        .count();
    before.len() - word
}
