//! One file's rows, built from the two sides of it.

use std::ops::Range;

use groove_text::Document;
use groove_types::{FileDiff, FileStatus, Hunk, Row, RowKind};

use super::{Aligned, Changes};
use crate::hunked::Hunked;
use crate::layout::Layout;
use crate::opened::MAX_SHOWN_BYTES;

/// One file's rows, from its two sides.
pub fn aligned(path: &str, before: &str, after: &str) -> Aligned {
    let (old, new) = (Document::plain(path, before), Document::plain(path, after));
    from_sides(path, &old, &new, &[])
}

/// One file's rows, from two documents already read.
pub fn from_sides(path: &str, old: &Document, new: &Document, opened: &[Range<u32>]) -> Aligned {
    let indent = new.indent().width();
    if old.bytes().max(new.bytes()) > MAX_SHOWN_BYTES {
        return Aligned {
            path: path.to_string(),
            hunked: Hunked {
                layout: Layout::long(),
                ..Hunked::default()
            },
            indent,
            long: true,
            ..Aligned::default()
        };
    }
    Aligned {
        path: path.to_string(),
        hunked: Hunked::of(old, new, opened),
        indent,
        long: false,
        old: old.clone(),
        new: new.clone(),
    }
}

/// Runs that touch stand as one.
pub(super) fn merge(spans: &mut Vec<Range<u32>>) {
    let mut at = 0;
    while at + 1 < spans.len() {
        match spans[at].end >= spans[at + 1].start {
            true => {
                spans[at].end = spans[at].end.max(spans[at + 1].end);
                spans.remove(at + 1);
            }
            false => at += 1,
        }
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

/// Files given as two texts each, an empty side added or deleted: the stream and each file's counts.
pub fn from_pairs(pairs: Vec<(String, String, String)>) -> (Vec<FileDiff>, Changes) {
    let mut files = Vec::new();
    let mut aligned_all = Vec::new();
    for (path, before, after) in pairs {
        let file = aligned(&path, &before, &after);
        let count = |side: fn(&Hunk) -> u32| file.hunked.hunks.iter().map(side).sum::<u32>();
        let status = match (before.is_empty(), after.is_empty()) {
            (true, _) => FileStatus::Added,
            (_, true) => FileStatus::Deleted,
            _ => FileStatus::Modified,
        };
        files.push(FileDiff {
            path,
            added: count(|one| one.after.end - one.after.start),
            deleted: count(|one| one.before.end - one.before.start),
            status,
            staged: None,
        });
        aligned_all.push(file);
    }
    (files, Changes::new(aligned_all))
}

/// One file's change as text: `+` added, `-` removed, unchanged runs said by their length.
pub fn unified(path: &str, before: &str, after: &str) -> String {
    let file = aligned(path, before, after);
    let mut out = format!("--- {path}\n");
    for (row, text) in file.rows().zip(file.texts()) {
        let line = match row.kind {
            RowKind::Context => format!("  {text}"),
            RowKind::Removed => format!("- {text}"),
            RowKind::Added => format!("+ {text}"),
            RowKind::Gap(lines) => format!("  … {lines} lines alike"),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}
