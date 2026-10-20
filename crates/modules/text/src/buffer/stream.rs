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

    /// The first `chars` characters taken out; each caret moves up with what it stood on.
    pub fn shed(&mut self, chars: usize) {
        let end = chars.min(self.doc.chars());
        if end == 0 {
            return;
        }
        let lines = self.doc.caret_of(end).line;
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
        self.forgot = self.revision;
    }
}
