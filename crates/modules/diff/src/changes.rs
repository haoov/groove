//! The whole change as one surface: every file's alignment, and no document behind it.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path;

use groove_git::Git;
use groove_text::{Document, Touched};
use groove_types::FileDiff;

mod aligned;
mod build;

pub use aligned::Aligned;
pub(crate) use build::text_of;
pub use build::{aligned, from_pairs, from_sides, unified};

use build::merge;

/// How many rows a file's head takes.
pub const HEAD_ROWS: usize = 2;

/// Where a row of the whole surface belongs.
#[derive(Debug, PartialEq, Eq)]
pub enum At<'a> {
    Head(&'a Aligned),
    Row(&'a Aligned, usize),
}

#[derive(Debug, Default)]
pub struct Changes {
    files: Vec<Aligned>,
    /// Where each file's own rows begin, the chrome above them included.
    starts: Vec<usize>,
    /// The files whose rows are hidden under their own head.
    shut: BTreeSet<String>,
    /// The one file that takes rows, when one is focused; the others take none.
    focus: Option<String>,
    /// The old-side lines each file's gaps have given up.
    opened: BTreeMap<String, Vec<Range<u32>>>,
    rows: usize,
    digits: usize,
}

impl Changes {
    pub fn new(files: Vec<Aligned>) -> Self {
        Self::indexed(files, BTreeSet::new(), None)
    }

    fn indexed(files: Vec<Aligned>, shut: BTreeSet<String>, focus: Option<String>) -> Self {
        let focus = focus.filter(|path| files.iter().any(|file| &file.path == path));
        let mut starts = Vec::with_capacity(files.len());
        let mut rows = 0;
        for file in &files {
            starts.push(rows);
            if focus.as_ref().is_some_and(|one| *one != file.path) {
                continue;
            }
            rows += match (focus.is_some(), shut.contains(&file.path)) {
                (true, _) => file.len(),
                (false, true) => HEAD_ROWS,
                (false, false) => file.len() + HEAD_ROWS,
            };
        }
        let digits = files.iter().map(widest).max().unwrap_or(1);
        Self {
            files,
            starts,
            shut,
            focus,
            opened: BTreeMap::new(),
            rows,
            digits,
        }
    }

    /// The change indexed again, its folds and focus kept.
    fn reindexed(&mut self) {
        let (shut, focus) = (std::mem::take(&mut self.shut), self.focus.take());
        let opened = std::mem::take(&mut self.opened);
        *self = Self::indexed(std::mem::take(&mut self.files), shut, focus);
        self.opened = opened;
    }

    /// Hides a file's rows under its own head, or shows them again.
    pub fn fold(&mut self, path: &str) {
        groove_types::toggle(&mut self.shut, path.to_string());
        self.reindexed();
    }

    /// Only this file takes rows; none shows every file again. A path the change lacks is none.
    pub fn focus(&mut self, path: Option<String>) {
        self.focus = path;
        self.reindexed();
    }

    /// The one file that takes rows, when one is focused.
    pub fn focused(&self) -> Option<&str> {
        self.focus.as_deref()
    }

    /// The rows a file's head takes; none while one file is focused.
    pub fn heads(&self) -> usize {
        match self.focus {
            Some(_) => 0,
            None => HEAD_ROWS,
        }
    }

    /// Whether this file takes rows: every file without a focus, else the focused one alone.
    fn placed_here(&self, path: &str) -> bool {
        self.focus.as_ref().is_none_or(|one| one == path)
    }

    /// A gap gives up a run of its old-side lines, and the file lines up again.
    pub fn open_gap(&mut self, path: &str, span: Range<u32>, old: &Document, new: &Document) {
        let mut opened = std::mem::take(&mut self.opened);
        let spans = opened.entry(path.to_string()).or_default();
        spans.push(span);
        spans.sort_by_key(|one| one.start);
        merge(spans);
        self.opened = opened;
        let Some(at) = self.files.iter().position(|file| file.path == path) else {
            return;
        };
        self.files[at] = from_sides(path, old, new, &self.opened[path]);
        self.reindexed();
    }

    /// What this file's gaps have already given up.
    pub fn opened_of(&self, path: &str) -> &[Range<u32>] {
        self.opened.get(path).map(Vec::as_slice).unwrap_or_default()
    }

    /// The folds and the focus carried over from the change this one replaces.
    pub fn refold(&mut self, shut: BTreeSet<String>, focus: Option<String>) {
        self.shut = shut
            .into_iter()
            .filter(|path| self.files.iter().any(|file| &file.path == path))
            .collect();
        self.focus = focus;
        self.reindexed();
    }

    pub fn folds(&self) -> BTreeSet<String> {
        self.shut.clone()
    }

    pub fn is_folded(&self, path: &str) -> bool {
        self.shut.contains(path)
    }

    /// How many of a file's rows the surface draws.
    pub fn shown(&self, file: &Aligned) -> usize {
        let shut = self.focus.is_none() && self.is_folded(&file.path);
        match shut || !self.placed_here(&file.path) {
            true => 0,
            false => file.len(),
        }
    }

    /// How wide the line numbers stand, over the whole change.
    pub fn digits(&self) -> usize {
        self.digits
    }

    /// How many rows the whole change stands.
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn files(&self) -> &[Aligned] {
        &self.files
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn at(&self, row: usize) -> Option<At<'_>> {
        if row >= self.rows {
            return None;
        }
        let at = self
            .starts
            .partition_point(|start| *start <= row)
            .checked_sub(1)?;
        let file = self.files.get(at)?;
        let (head, heads) = (self.starts[at], self.heads());
        match row < head + heads {
            true => Some(At::Head(file)),
            false => Some(At::Row(file, row - head - heads)),
        }
    }

    /// The row a file's new-side line stands on; a folded or an unfocused file shows none.
    pub fn row_of(&self, path: &str, line: u32) -> Option<usize> {
        let at = self.files.iter().position(|file| file.path == path)?;
        if self.shown(&self.files[at]) == 0 || !self.placed_here(path) {
            return None;
        }
        let head = self.starts[at];
        let row = self.files[at].hunked.layout.position(line)?;
        Some(head + self.heads() + row)
    }

    /// The row this file's own head sits on; an unfocused file has none.
    pub fn head_of(&self, path: &str) -> Option<usize> {
        let at = self.files.iter().position(|file| file.path == path)?;
        self.placed_here(path).then(|| self.starts[at])
    }

    /// Every file that takes rows, with the row its own block begins on, and the rows it shows.
    pub fn placed(&self) -> impl Iterator<Item = (usize, usize, &Aligned)> {
        self.starts
            .iter()
            .copied()
            .zip(self.files.iter())
            .filter(|(_, file)| self.placed_here(&file.path))
            .map(|(start, file)| (start, self.shown(file), file))
    }

    pub fn get(&self, path: &str) -> Option<&Aligned> {
        self.files.iter().find(|file| file.path == path)
    }

    /// One edit of a file's new side, moved into its hunks with nothing diffed again.
    pub fn edited(&mut self, path: &str, edit: Touched, new: &Document) {
        let Some(at) = self.files.iter().position(|one| one.path == path) else {
            return;
        };
        let opened = self.opened.get(path).map_or(&[][..], Vec::as_slice);
        self.files[at].edited(edit, new, opened);
        self.reindexed();
    }

    /// One file aligned again, for a buffer that has moved under it.
    pub fn replace(&mut self, file: Aligned) {
        let Some(at) = self.files.iter().position(|one| one.path == file.path) else {
            return;
        };
        self.files[at] = file;
        self.reindexed();
    }
}

/// The digits the highest line number of a file takes.
fn widest(file: &Aligned) -> usize {
    (file.hunked.layout.highest() + 1).to_string().len()
}

/// Every changed file aligned, the HEAD sides read in one git process.
pub async fn changes(dir: &Path, files: &[FileDiff], rev: &str) -> groove_types::Result<Changes> {
    let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
    let heads = Git::at(dir).blobs(rev, &paths).await?;
    let mut out = Vec::with_capacity(paths.len());
    for path in &paths {
        let before = crate::committed(heads.get(path).map(String::as_str).unwrap_or_default());
        let after = crate::working(dir, path).await;
        let (before, after) = crate::paired(before, &after);
        out.push(aligned(path, before, after));
    }
    Ok(Changes::new(out))
}
