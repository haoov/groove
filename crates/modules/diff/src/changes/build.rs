//! One file's rows, built from the two sides of it.

use std::ops::Range;

use groove_text::Document;
use groove_types::{Row, RowKind};

use super::Aligned;
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
