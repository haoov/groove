//! The whole change as one surface: every file's alignment, and no document behind it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use groove_git::Git;
use groove_text::Document;
use groove_types::{FileDiff, LineMark, Row, RowKind};

use crate::alignment::{CONTEXT, align, marks};
use crate::opened::MAX_SHOWN_BYTES;

/// One changed file: how its sides line up, and what each row shows.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Aligned {
    pub path: String,
    pub rows: Vec<Row>,
    /// One per row, in the row's own order.
    pub lines: Vec<String>,
    pub marks: BTreeMap<u32, LineMark>,
    /// How wide a tab reads in this file.
    pub indent: usize,
    /// Too long to align; the surface says so instead of drawing it.
    pub long: bool,
}

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
}

/// Where a row of the whole surface belongs.
#[derive(Debug, PartialEq, Eq)]
pub enum At<'a> {
    /// The row naming the directory the files below it share.
    Band(&'a Aligned),
    Head(&'a Aligned),
    Row(&'a Aligned, usize),
}

#[derive(Debug, Default)]
pub struct Changes {
    files: Vec<Aligned>,
    /// Where each file's own rows begin, the chrome above them included.
    starts: Vec<usize>,
    /// Whether each file opens a directory of its own.
    bands: Vec<bool>,
    /// The files whose rows are hidden under their own head.
    shut: BTreeSet<String>,
    rows: usize,
    digits: usize,
}

impl Changes {
    pub fn new(files: Vec<Aligned>) -> Self {
        Self::indexed(files, BTreeSet::new())
    }

    fn indexed(files: Vec<Aligned>, shut: BTreeSet<String>) -> Self {
        let mut starts = Vec::with_capacity(files.len());
        let mut bands = Vec::with_capacity(files.len());
        let mut rows = 0;
        let mut dir = None;
        for file in &files {
            let band = !file.dir().is_empty() && dir != Some(file.dir());
            dir = Some(file.dir());
            starts.push(rows);
            bands.push(band);
            let shown = match shut.contains(&file.path) {
                true => 0,
                false => file.rows.len(),
            };
            rows += shown + 1 + usize::from(band);
        }
        let digits = files.iter().map(widest).max().unwrap_or(1);
        Self {
            files,
            starts,
            bands,
            shut,
            rows,
            digits,
        }
    }

    /// Hides a file's rows under its own head, or shows them again.
    pub fn fold(&mut self, path: &str) {
        let mut shut = std::mem::take(&mut self.shut);
        if !shut.remove(path) {
            shut.insert(path.to_string());
        }
        *self = Self::indexed(std::mem::take(&mut self.files), shut);
    }

    /// The folds carried over from the change this one replaces.
    pub fn refold(&mut self, shut: BTreeSet<String>) {
        let kept = shut
            .into_iter()
            .filter(|path| self.files.iter().any(|file| &file.path == path))
            .collect();
        *self = Self::indexed(std::mem::take(&mut self.files), kept);
    }

    pub fn folds(&self) -> BTreeSet<String> {
        self.shut.clone()
    }

    pub fn is_folded(&self, path: &str) -> bool {
        self.shut.contains(path)
    }

    /// How many of a file's rows the surface draws.
    pub fn shown(&self, file: &Aligned) -> usize {
        match self.is_folded(&file.path) {
            true => 0,
            false => file.rows.len(),
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
        let band = usize::from(self.bands[at]);
        let head = self.starts[at] + band;
        match row {
            _ if row < head => Some(At::Band(file)),
            _ if row == head => Some(At::Head(file)),
            _ => Some(At::Row(file, row - head - 1)),
        }
    }

    /// The row this file's own band or head sits on.
    pub fn head_of(&self, path: &str) -> Option<usize> {
        let at = self.files.iter().position(|file| file.path == path)?;
        Some(self.starts[at])
    }

    /// Every file with the row its own block begins on, and the rows it shows.
    pub fn placed(&self) -> impl Iterator<Item = (usize, usize, &Aligned)> {
        self.starts
            .iter()
            .copied()
            .zip(self.files.iter())
            .map(|(start, file)| (start, self.shown(file), file))
    }

    pub fn get(&self, path: &str) -> Option<&Aligned> {
        self.files.iter().find(|file| file.path == path)
    }

    /// One file aligned again, for a buffer that has moved under it.
    pub fn replace(&mut self, file: Aligned) {
        let Some(at) = self.files.iter().position(|one| one.path == file.path) else {
            return;
        };
        self.files[at] = file;
        let shut = std::mem::take(&mut self.shut);
        *self = Self::indexed(std::mem::take(&mut self.files), shut);
    }
}

/// The digits the highest line number of a file takes.
fn widest(file: &Aligned) -> usize {
    let highest = file
        .rows
        .iter()
        .filter_map(|row| row.old.max(row.new))
        .max()
        .unwrap_or(0);
    (highest + 1).to_string().len()
}

/// Every changed file aligned, the HEAD sides read in one git process.
pub async fn changes(dir: &Path, files: &[FileDiff]) -> Changes {
    let paths: Vec<String> = files.iter().map(|file| file.path.clone()).collect();
    let heads = Git::at(dir).blobs("HEAD", &paths).await.unwrap_or_default();
    let aligned = paths
        .iter()
        .map(|path| {
            let before = heads.get(path).map(String::as_str).unwrap_or_default();
            let after = std::fs::read_to_string(dir.join(path)).unwrap_or_default();
            aligned(path, before, &after)
        })
        .collect();
    Changes::new(aligned)
}

/// One file's rows, from its two sides.
pub fn aligned(path: &str, before: &str, after: &str) -> Aligned {
    let (old, new) = (Document::plain(path, before), Document::plain(path, after));
    let indent = new.indent().width();
    if old.bytes().max(new.bytes()) > MAX_SHOWN_BYTES {
        return Aligned {
            path: path.to_string(),
            rows: vec![Row {
                old: None,
                new: None,
                kind: RowKind::Gap(0),
            }],
            lines: vec![String::new()],
            indent,
            long: true,
            ..Aligned::default()
        };
    }
    let rows = align(&old, &new, CONTEXT);
    Aligned {
        lines: rows.iter().map(|row| text_of(&old, &new, row)).collect(),
        marks: marks(&rows),
        path: path.to_string(),
        rows,
        indent,
        long: false,
    }
}

/// What a row shows: its own side's line, or how many lines a gap hides.
pub(crate) fn text_of(old: &Document, new: &Document, row: &Row) -> String {
    if let RowKind::Gap(lines) = row.kind {
        return format!("\u{2026} {lines} lines");
    }
    let line = match (row.new, row.old) {
        (Some(at), _) => new.line(at as usize),
        (None, Some(at)) => old.line(at as usize),
        (None, None) => None,
    };
    line.unwrap_or_default().to_string()
}
