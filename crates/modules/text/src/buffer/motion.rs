//! Where a motion takes a caret, and what a selection holds.

use groove_types::{Caret, Motion, Selection};

use super::Buffer;
use crate::words::class;

impl Buffer {
    /// Moves every caret. Extending leaves the anchor where it is.
    pub(super) fn go(&mut self, motion: Motion, extend: bool) {
        self.history.close();
        self.carets = self
            .carets
            .iter()
            .map(|one| self.moved(one, motion, extend))
            .collect();
    }

    pub(super) fn moved(&self, one: &Selection, motion: Motion, extend: bool) -> Selection {
        let head = self.clamped(self.towards(one, motion));
        match extend {
            true => Selection {
                anchor: one.anchor,
                head,
            },
            false => Selection::at(head),
        }
    }

    /// Where a motion takes the caret, before it is clamped to the text.
    pub(super) fn towards(&self, one: &Selection, motion: Motion) -> Caret {
        let at = self.doc.char_of(one.head);
        let head = one.head;
        match motion {
            Motion::Left => self.doc.caret_of(at.saturating_sub(1)),
            Motion::Right => self.doc.caret_of(at + 1),
            Motion::Up => Caret::new(head.line.saturating_sub(1), head.column),
            Motion::Down => Caret::new(head.line + 1, head.column),
            Motion::LineStart => Caret::new(head.line, 0),
            Motion::LineEnd => Caret::new(head.line, self.doc.line_chars(head.line)),
            Motion::To(caret) => caret,
        }
    }

    /// Every caret holds what `pick` makes of the line it is on.
    pub(super) fn hold(&mut self, pick: impl Fn(&Self, Caret) -> Selection) {
        self.history.close();
        self.carets = self.carets.iter().map(|one| pick(self, one.head)).collect();
    }

    /// The run around the caret: a word, the spaces between words, or a run of marks.
    pub(super) fn word(&self, caret: Caret) -> Selection {
        let Some(line) = self.doc.line(caret.line) else {
            return Selection::at(caret);
        };
        let chars: Vec<char> = line.chars().collect();
        let at = caret.column.min(chars.len().saturating_sub(1));
        let Some(kind) = chars.get(at).copied().map(class) else {
            return Selection::at(caret);
        };
        let same = |column: &usize| chars.get(*column).copied().map(class) == Some(kind);
        let from = (0..=at).rev().take_while(&same).last().unwrap_or(at);
        let to = (at..chars.len()).take_while(&same).last().unwrap_or(at) + 1;
        Selection {
            anchor: Caret::new(caret.line, from),
            head: Caret::new(caret.line, to),
        }
    }

    /// The line and the break that ends it, so taking it out takes the row away.
    pub(super) fn whole_line(&self, caret: Caret) -> Selection {
        let last = caret.line + 1 >= self.doc.lines();
        let head = match last {
            true => Caret::new(caret.line, self.doc.line_chars(caret.line)),
            false => Caret::new(caret.line + 1, 0),
        };
        Selection {
            anchor: Caret::new(caret.line, 0),
            head,
        }
    }

    /// The whole document, selected by one caret.
    pub(super) fn all(&mut self) {
        self.history.close();
        let end = self.doc.caret_of(self.doc.chars());
        self.carets = vec![Selection {
            anchor: Caret::default(),
            head: end,
        }];
    }

    /// A caret at a place the text has.
    pub(super) fn clamped(&self, caret: Caret) -> Caret {
        self.doc.caret_of(self.doc.char_of(caret))
    }

    /// What a selection covers, in characters.
    pub(super) fn range(&self, one: &Selection) -> std::ops::Range<usize> {
        let (from, to) = one.ends();
        self.doc.char_of(from)..self.doc.char_of(to)
    }

    pub(super) fn first(&self) -> Selection {
        self.carets.first().copied().unwrap_or_default()
    }
}
