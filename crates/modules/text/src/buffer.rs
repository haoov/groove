//! A document being edited: what its carets own, what it owes the disk, its history.

use groove_types::{Caret, Edit, Highlight, Motion, Selection};

use crate::history::{Change, History};
use crate::{Document, Settled};

/// What a character belongs to, for picking out a word.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Word,
    Space,
    Mark,
}

fn class(c: char) -> Class {
    match c {
        _ if c.is_alphanumeric() || c == '_' => Class::Word,
        _ if c.is_whitespace() => Class::Space,
        _ => Class::Mark,
    }
}

pub struct Buffer {
    doc: Document,
    /// One per caret. Several carets edit at once; today there is one.
    carets: Vec<Selection>,
    history: History,
    dirty: bool,
    /// Bumped by every change, so a job knows whether its answer is still wanted.
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

    pub fn spans(&self, at: usize) -> Vec<Highlight> {
        self.doc.spans(at)
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

    /// The disk has what the buffer holds.
    pub fn saved(&mut self) {
        self.dirty = false;
        self.history.close();
    }

    /// Reads the colours again, when nothing has changed since `revision`.
    pub fn recolour(&mut self, revision: u64) {
        if revision == self.revision {
            self.doc.recolour();
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
            Edit::Newline => self.write("\n"),
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

    /// Puts `text` in at every caret, over whatever it had selected.
    fn write(&mut self, text: &str) {
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
    fn erase(&mut self, towards: Motion) {
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
    fn one_character(&self, caret: Caret, towards: Motion) -> Option<Change> {
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

    /// One change per caret, in reverse reading order so the offsets ahead of each
    /// change are still the ones it was built from.
    fn at_each(&self, change: impl Fn(&Self, &Selection) -> Option<Change>) -> Vec<Change> {
        let mut carets = self.carets.clone();
        carets.sort_by_key(|one| std::cmp::Reverse(one.ends().0));
        carets.iter().filter_map(|one| change(self, one)).collect()
    }

    fn apply(&mut self, changes: Vec<Change>) {
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

    fn step(&mut self, take: impl Fn(&mut History) -> Option<Vec<Change>>) {
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
    fn change(&mut self, change: &Change) {
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

    /// Moves every caret. Extending leaves the anchor where it is.
    fn go(&mut self, motion: Motion, extend: bool) {
        self.history.close();
        self.carets = self
            .carets
            .iter()
            .map(|one| self.moved(one, motion, extend))
            .collect();
    }

    fn moved(&self, one: &Selection, motion: Motion, extend: bool) -> Selection {
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
    fn towards(&self, one: &Selection, motion: Motion) -> Caret {
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
    fn hold(&mut self, pick: impl Fn(&Self, Caret) -> Selection) {
        self.history.close();
        self.carets = self.carets.iter().map(|one| pick(self, one.head)).collect();
    }

    /// The run of characters of one kind around the caret: a word, the spaces
    /// between words, or a run of marks.
    fn word(&self, caret: Caret) -> Selection {
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
    fn whole_line(&self, caret: Caret) -> Selection {
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
    fn all(&mut self) {
        self.history.close();
        let end = self.doc.caret_of(self.doc.chars());
        self.carets = vec![Selection {
            anchor: Caret::default(),
            head: end,
        }];
    }

    /// A caret at a place the text has.
    fn clamped(&self, caret: Caret) -> Caret {
        self.doc.caret_of(self.doc.char_of(caret))
    }

    /// What a selection covers, in characters.
    fn range(&self, one: &Selection) -> std::ops::Range<usize> {
        let (from, to) = one.ends();
        self.doc.char_of(from)..self.doc.char_of(to)
    }

    fn first(&self) -> Selection {
        self.carets.first().copied().unwrap_or_default()
    }
}
