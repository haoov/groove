//! The change: every file's alignment, one of them focused on the surface.

use std::collections::BTreeMap;
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

#[derive(Debug, Default)]
pub struct Changes {
    files: Vec<Aligned>,
    /// The one file that takes rows; with none, no file does.
    focus: Option<String>,
    /// The old-side lines each file's gaps have given up.
    opened: BTreeMap<String, Vec<Range<u32>>>,
    rows: usize,
    digits: usize,
}

impl Changes {
    pub fn new(files: Vec<Aligned>) -> Self {
        Self::indexed(files, None)
    }

    fn indexed(files: Vec<Aligned>, focus: Option<String>) -> Self {
        let focus = focus.filter(|path| files.iter().any(|file| &file.path == path));
        let focused = files.iter().find(|file| Some(&file.path) == focus.as_ref());
        let rows = focused.map_or(0, Aligned::len);
        let digits = files.iter().map(widest).max().unwrap_or(1);
        Self {
            files,
            focus,
            opened: BTreeMap::new(),
            rows,
            digits,
        }
    }

    /// The change indexed again, its focus kept.
    fn reindexed(&mut self) {
        let focus = self.focus.take();
        let opened = std::mem::take(&mut self.opened);
        *self = Self::indexed(std::mem::take(&mut self.files), focus);
        self.opened = opened;
    }

    /// Only this file takes rows; none takes none. A path the change lacks is none.
    pub fn focus(&mut self, path: Option<String>) {
        self.focus = path;
        self.reindexed();
    }

    /// The one file that takes rows, when one is focused.
    pub fn focused(&self) -> Option<&str> {
        self.focus.as_deref()
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

    /// How wide the line numbers stand, over the whole change.
    pub fn digits(&self) -> usize {
        self.digits
    }

    /// How many rows the focused file stands.
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn files(&self) -> &[Aligned] {
        &self.files
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The focused file, and the row of its own this row of the surface stands on.
    pub fn at(&self, row: usize) -> Option<(&Aligned, usize)> {
        let file = self.get(self.focus.as_deref()?)?;
        (row < self.rows).then_some((file, row))
    }

    /// The row a file's new-side line stands on; a file not focused shows none.
    pub fn row_of(&self, path: &str, line: u32) -> Option<usize> {
        if self.focus.as_deref() != Some(path) {
            return None;
        }
        self.get(path)?.hunked.layout.position(line)
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
