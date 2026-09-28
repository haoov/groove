//! What a change does to the buffer: the text it writes, and the history it keeps.

use groove_types::{Caret, Motion, Selection};

use super::Buffer;
use crate::history::{Change, History};

impl Buffer {
    /// A new line at every caret, indented like the line above.
    pub(super) fn broke(&mut self) {
        let changes = self.at_each(|buffer, one| {
            let range = buffer.range(one);
            Some(Change {
                at: range.start,
                removed: buffer.doc.slice(range),
                inserted: format!("\n{}", buffer.leading(one.head.line)),
            })
        });
        self.apply(changes);
    }

    /// The whitespace a line opens with, up to where its text begins.
    pub(super) fn leading(&self, line: usize) -> String {
        self.doc
            .line(line)
            .unwrap_or_default()
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }

    /// Puts `text` in at every caret, over whatever it had selected.
    pub(super) fn write(&mut self, text: &str) {
        let changes = self.at_each(|buffer, one| {
            Some(Change {
                at: buffer.range(one).start,
                removed: buffer.doc.slice(buffer.range(one)),
                inserted: text.to_string(),
            })
        });
        self.apply(changes);
    }

    /// Takes out what is selected, or the character the motion points at.
    pub(super) fn erase(&mut self, towards: Motion) {
        let changes = self.at_each(|buffer, one| match one.is_empty() {
            false => Some(Change {
                at: buffer.range(one).start,
                removed: buffer.doc.slice(buffer.range(one)),
                inserted: String::new(),
            }),
            true => buffer.one_character(one.head, towards),
        });
        self.apply(changes);
    }

    /// The change that takes the character beside `caret` out.
    pub(super) fn one_character(&self, caret: Caret, towards: Motion) -> Option<Change> {
        let at = self.doc.char_of(caret);
        let (start, end) = match towards {
            Motion::Left => (at.checked_sub(1)?, at),
            _ => (at, (at + 1).min(self.doc.chars())),
        };
        (start < end).then(|| Change {
            at: start,
            removed: self.doc.slice(start..end),
            inserted: String::new(),
        })
    }

    /// One change per caret, last first.
    pub(super) fn at_each(
        &self,
        change: impl Fn(&Self, &Selection) -> Option<Change>,
    ) -> Vec<Change> {
        let mut carets = self.carets.clone();
        carets.sort_by_key(|one| std::cmp::Reverse(one.ends().0));
        carets.iter().filter_map(|one| change(self, one)).collect()
    }

    pub(super) fn apply(&mut self, changes: Vec<Change>) {
        if changes.is_empty() {
            return;
        }
        let carets = changes
            .iter()
            .map(|change| {
                self.change(change);
                Selection::at(self.doc.caret_of(change.after()))
            })
            .collect::<Vec<Selection>>();
        self.carets = carets.into_iter().rev().collect();
        self.history.did(changes);
    }

    pub(super) fn step(&mut self, take: impl Fn(&mut History) -> Option<Vec<Change>>) {
        let Some(changes) = take(&mut self.history) else {
            return;
        };
        let mut carets = Vec::new();
        for change in &changes {
            self.change(change);
            carets.push(Selection::at(self.doc.caret_of(change.after())));
        }
        self.carets = carets;
    }

    /// Puts a change into the document.
    pub(super) fn change(&mut self, change: &Change) {
        let breaks = |text: &str| text.matches('\n').count() as u32;
        self.touched.push(super::Touched {
            line: self.doc.caret_of(change.at).line as u32,
            gone: 1 + breaks(&change.removed),
            came: 1 + breaks(&change.inserted),
        });
        let end = change.at + change.removed.chars().count();
        if !change.removed.is_empty() {
            self.doc.remove(change.at..end);
        }
        if !change.inserted.is_empty() {
            self.doc.insert(change.at, &change.inserted);
        }
        self.dirty = true;
        self.revision += 1;
    }
}
