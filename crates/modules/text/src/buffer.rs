//! A document being edited: what its carets own, what it owes the disk, its history.

use groove_types::{Caret, Edit, Motion, Selection};

use crate::history::History;
use crate::{Colours, Document, Settled};

mod edits;
mod motion;

pub struct Buffer {
    doc: Document,
    /// One per caret. Several carets edit at once; today there is one.
    carets: Vec<Selection>,
    history: History,
    dirty: bool,
    /// Bumped by every change.
    revision: u64,
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("carets", &self.carets)
            .field("dirty", &self.dirty)
            .field("revision", &self.revision)
            .finish()
    }
}

impl Default for Buffer {
    /// An empty buffer of no language, for what is typed outside a file.
    fn default() -> Self {
        Self::new(Document::new("", ""))
    }
}

impl Buffer {
    pub fn new(doc: Document) -> Self {
        Self {
            doc,
            carets: vec![Selection::default()],
            history: History::default(),
            dirty: false,
            revision: 0,
        }
    }

    pub fn document(&self) -> &Document {
        &self.doc
    }

    pub fn lines(&self) -> usize {
        self.doc.lines()
    }

    pub fn line(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        self.doc.line(at)
    }

    pub fn colours(&self, lines: std::ops::Range<usize>) -> Colours {
        self.doc.colours(lines)
    }

    pub fn text(&self) -> String {
        self.doc.text()
    }

    /// Where the first caret is.
    pub fn caret(&self) -> Caret {
        self.first().head
    }

    pub fn selections(&self) -> &[Selection] {
        &self.carets
    }

    /// The text every caret has selected, in reading order.
    pub fn selected(&self) -> String {
        let mut carets = self.carets.clone();
        carets.sort_by_key(|one| one.ends().0);
        carets
            .iter()
            .filter(|one| !one.is_empty())
            .map(|one| self.doc.slice(self.range(one)))
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Puts the caret back where it was in another read of the same file.
    pub fn follow(&mut self, caret: Caret) {
        self.carets = vec![Selection::at(self.clamped(caret))];
    }

    /// Puts one caret where it was and gives it what it held.
    pub fn holding(&mut self, held: Selection) {
        self.carets = vec![Selection {
            anchor: self.clamped(held.anchor),
            head: self.clamped(held.head),
        }];
    }

    /// The disk has what the buffer holds.
    pub fn saved(&mut self) {
        self.dirty = false;
        self.history.close();
    }

    /// Parses again, when nothing has changed since `revision`.
    pub fn reparse(&mut self, revision: u64) {
        if revision == self.revision {
            self.doc.reparse();
        }
    }

    /// What a job settled for `revision`, if the buffer has not moved on.
    pub fn settled(&mut self, settled: Settled, revision: u64) -> bool {
        if revision != self.revision {
            return false;
        }
        self.doc.install(settled);
        true
    }

    pub fn edit(&mut self, edit: &Edit) {
        match edit {
            Edit::Insert(text) => self.write(text),
            Edit::Newline => self.broke(),
            Edit::Indent => self.write(&self.doc.indent().text()),
            Edit::Backspace => self.erase(Motion::Left),
            Edit::Delete => self.erase(Motion::Right),
            Edit::Move(motion) => self.go(*motion, false),
            Edit::Extend(motion) => self.go(*motion, true),
            Edit::SelectAll => self.all(),
            Edit::SelectWord => self.hold(Self::word),
            Edit::SelectLine => self.hold(Self::whole_line),
            Edit::Undo => self.step(History::undo),
            Edit::Redo => self.step(History::redo),
        }
    }
}
