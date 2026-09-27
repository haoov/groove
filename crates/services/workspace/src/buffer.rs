//! The open file's buffer: what a read, a derive and a save do to it.

use groove_types::Selection;

use crate::{Derived, Opened, State, by_line, from_documents};

impl State {
    /// Installs what a derive found, when the buffer is still the one it read.
    pub fn derived(&mut self, path: &str, read: Derived, revision: u64) {
        let Some(open) = self.opened.as_mut() else {
            return;
        };
        if open.path != path || !open.new.settled(read.settled, revision) {
            return;
        }
        open.rows = read.aligned.rows.clone();
        open.marks = read.aligned.marks.clone();
        open.words = by_line(&read.aligned.rows, &read.aligned.words);
        self.changes.replace(read.aligned);
    }

    /// The buffer marked as what the disk holds.
    pub fn saved(&mut self, path: &str) {
        if let Some(open) = self.opened.as_mut().filter(|open| open.path == path) {
            open.new.saved();
        }
    }

    /// The two sides of a file the stream holds, aligned again for the buffer.
    pub fn in_hand(&self, path: &str) -> Option<Opened> {
        if self.commit.is_some() {
            return None;
        }
        let painted = self.coloured.get(path)?;
        Some(from_documents(
            path,
            painted.old.clone(),
            painted.new.clone(),
        ))
    }

    /// The file read, with the caret it had while it was the same file.
    pub fn arrived(&mut self, file: Opened, at: Option<Selection>) {
        match self.reads_the_same(&file) {
            true => self.refreshed(file, at),
            false => self.replaced(file, at),
        }
    }

    /// Whether the buffer in hand already holds the text this read found.
    fn reads_the_same(&self, file: &Opened) -> bool {
        self.opened
            .as_ref()
            .is_some_and(|open| open.path == file.path && open.new.text() == file.new.text())
    }

    /// The read brought back the text the buffer holds: the buffer stays, with its history.
    fn refreshed(&mut self, file: Opened, at: Option<Selection>) {
        let Some(open) = self.opened.as_mut() else {
            return;
        };
        open.old = file.old;
        open.rows = file.rows;
        open.marks = file.marks;
        open.words = file.words;
        open.long = file.long;
        open.new.saved();
        if let Some(held) = at {
            open.new.holding(held);
        }
    }

    /// A different text: the buffer gives way, keeping only where the caret was.
    fn replaced(&mut self, mut file: Opened, at: Option<Selection>) {
        let held = at.or_else(|| {
            self.opened
                .as_ref()
                .filter(|open| open.path == file.path)
                .map(|open| Selection::at(open.new.caret()))
        });
        if let Some(held) = held {
            file.new.holding(held);
        }
        self.opened = Some(file);
        self.moved();
    }
}
