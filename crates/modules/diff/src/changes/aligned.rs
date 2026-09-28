//! One changed file: its hunks, the rows they stand for, and both sides to read a row from.

use std::ops::Range;

use groove_text::{Document, Touched};
use groove_types::{Row, RowKind};

use crate::hunked::Hunked;

/// One changed file: how its sides line up, and what each row shows.
#[derive(Debug)]
pub struct Aligned {
    pub path: String,
    pub hunked: Hunked,
    /// How wide a tab reads in this file.
    pub indent: usize,
    /// Too long to align; the surface says so instead of drawing it.
    pub long: bool,
    pub(crate) old: Document,
    pub(crate) new: Document,
}

impl Default for Aligned {
    fn default() -> Self {
        Self {
            path: String::new(),
            hunked: Hunked::default(),
            indent: 0,
            long: false,
            old: Document::plain("", ""),
            new: Document::plain("", ""),
        }
    }
}

impl PartialEq for Aligned {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.hunked == other.hunked
            && (self.indent, self.long) == (other.indent, other.long)
            && self.old.text() == other.old.text()
            && self.new.text() == other.new.text()
    }
}

impl Eq for Aligned {}

impl Aligned {
    /// The directory the file sits in, from the worktree root.
    pub fn dir(&self) -> &str {
        match self.path.rsplit_once('/') {
            Some((dir, _)) => dir,
            None => "",
        }
    }

    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// How many rows the file stands.
    pub fn len(&self) -> usize {
        self.hunked.layout.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hunked.layout.is_empty()
    }

    pub fn row(&self, at: usize) -> Option<Row> {
        self.hunked.layout.row(at)
    }

    pub fn rows(&self) -> impl Iterator<Item = Row> + '_ {
        self.hunked.layout.all()
    }

    /// Every row's text, in order.
    pub fn texts(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.len()).map(|at| self.text(at))
    }

    /// What a row shows: its own side's line, or how many lines a gap hides.
    pub fn text(&self, at: usize) -> String {
        self.row(at)
            .map(|row| super::text_of(&self.old, &self.new, &row))
            .unwrap_or_default()
    }

    /// One edit of the new side, `new` being its text after it; `opened` as its gaps stand.
    pub fn edited(&mut self, edit: Touched, new: &Document, opened: &[Range<u32>]) {
        self.hunked.edited(edit, &self.old, new, opened);
        self.new = new.clone();
    }

    /// The columns a row draws that the row it pairs with does not.
    pub fn words(&self, at: usize) -> Option<&Vec<Range<usize>>> {
        let row = self.row(at)?;
        match row.kind {
            RowKind::Removed => self.hunked.words.old.get(&row.old?),
            RowKind::Added => self.hunked.words.new.get(&row.new?),
            _ => None,
        }
    }
}
