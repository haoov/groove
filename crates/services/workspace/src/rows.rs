//! The whole change as rows: which files the screen needs, and what a gap gives up.

use std::collections::BTreeMap;
use std::ops::Range;

use groove_types::RowKind;

use crate::{At, Document, Opened, Painted, State};

/// How far past the rows on screen a file is read at.
const AHEAD: usize = 200;

/// How many lines one click of a gap gives up.
const STEP: u32 = 20;

/// Which end of a gap gives its lines up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    Up,
    Down,
    All,
}

impl State {
    /// The rows on screen: far files let go, near ones not yet read returned.
    pub fn show(&mut self, rows: Range<usize>) -> Vec<String> {
        if self.showing == rows {
            return Vec::new();
        }
        self.showing = rows.clone();
        let wanted = self.over(rows.start.saturating_sub(AHEAD)..rows.end + AHEAD);
        self.coloured.retain(|path, _| wanted.contains(path));
        wanted
            .into_iter()
            .filter(|path| !self.coloured.contains_key(path))
            .collect()
    }

    /// The paths whose rows `rows` covers.
    pub fn over(&self, rows: Range<usize>) -> Vec<String> {
        let mut paths: Vec<String> = Vec::new();
        for row in rows {
            let path = match self.changes.at(row) {
                Some(At::Head(file) | At::Row(file, _)) => &file.path,
                None => break,
            };
            if paths.last() != Some(path) {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// The gap on this row gives up its lines, from one end or whole.
    pub fn open_gap_at(&mut self, row: usize, way: Way) {
        let Some((path, span)) = self.gap_at(row) else {
            return;
        };
        let wanted = match way {
            Way::All => span,
            Way::Down => span.start..(span.start + STEP).min(span.end),
            Way::Up => span.end.saturating_sub(STEP).max(span.start)..span.end,
        };
        self.open_gap(&path, wanted);
    }

    /// The file a gap row belongs to, and the old-side lines it hides.
    fn gap_at(&self, row: usize) -> Option<(String, Range<u32>)> {
        let At::Row(file, at) = self.changes.at(row)? else {
            return None;
        };
        let RowKind::Gap(lines) = file.row(at)?.kind else {
            return None;
        };
        let start = (0..at)
            .rev()
            .find_map(|one| file.row(one)?.old)
            .map(|old| old + 1)
            .unwrap_or_default();
        Some((file.path.clone(), start..start + lines))
    }

    /// A gap gives up a run of its old-side lines, from the documents already read.
    pub fn open_gap(&mut self, path: &str, span: Range<u32>) {
        let open = match (&self.worktree, self.commit.is_none()) {
            (Some(worktree), true) => self.buffers.get(worktree).and_then(|one| one.get(path)),
            _ => None,
        };
        if let Some((old, new)) = sides_of(open, &self.coloured, path) {
            self.changes.open_gap(path, span, old, new);
        }
    }

    /// Both sides of a file: its buffer's while it is open, else the ones read.
    pub fn sides(&self, path: &str) -> Option<(&Document, &Document)> {
        let open = self.buffer(path).filter(|_| self.commit.is_none());
        sides_of(open, &self.coloured, path)
    }
}

fn sides_of<'a>(
    opened: Option<&'a Opened>,
    coloured: &'a BTreeMap<String, Painted>,
    path: &str,
) -> Option<(&'a Document, &'a Document)> {
    if let Some(open) = opened {
        return Some((&open.old, open.new.document()));
    }
    let painted = coloured.get(path)?;
    Some((&painted.old, &painted.new))
}
