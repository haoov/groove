//! A text that only grows at its end and loses lines at its start, outside the history: a log.

use groove_types::{Caret, Selection};

use super::Buffer;

impl Buffer {
    /// `text` added after the last character.
    pub fn append(&mut self, text: &str) {
        let end = self.doc.chars();
        self.doc.insert(end, text);
        self.revision += 1;
    }

    /// The first `lines` lines taken out; each caret moves up with what it stood on.
    pub fn shed(&mut self, lines: usize) {
        let lines = lines.min(self.doc.lines());
        if lines == 0 {
            return;
        }
        let end = self.doc.char_of(Caret::new(lines, 0));
        self.doc.remove(0..end);
        let up = |at: Caret| match at.line.checked_sub(lines) {
            Some(line) => Caret::new(line, at.column),
            None => Caret::new(0, 0),
        };
        for one in &mut self.carets {
            *one = Selection {
                anchor: up(one.anchor),
                head: up(one.head),
            };
        }
        self.revision += 1;
    }
}
