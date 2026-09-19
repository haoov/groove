use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use groove_git::Git;
use groove_text::{Buffer, Document, Settled};
use groove_types::{LineMark, Result, Row};

use crate::alignment::{CONTEXT, align, by_line, marks, words};
use crate::changes::{Aligned, text_of};

/// Above this a file is listed as changed and not shown.
pub const MAX_SHOWN_BYTES: usize = 2 << 20;

/// A file as the diff shows it: both sides, and how they line up.
#[derive(Debug)]
pub struct Opened {
    pub path: String,
    pub old: Document,
    /// The side the user edits.
    pub new: Buffer,
    pub rows: Vec<Row>,
    /// What the change did to each line of the new file.
    pub marks: BTreeMap<u32, LineMark>,
    /// The columns of a changed line the old one does not have.
    pub words: BTreeMap<u32, Vec<Range<usize>>>,
    /// Too long to align; the view says so instead of drawing it.
    pub long: bool,
}

/// The file at `path` as it is on disk against the version in HEAD.
pub async fn opened(dir: &Path, path: &str) -> Result<Opened> {
    let before = committed(dir, path).await;
    let after = working(dir, path);
    Ok(from_text(path, &before, &after))
}

/// The file read again, against the HEAD side already read for it. Only a git
/// command changes that side, and nothing here runs one.
pub fn reopened(dir: &Path, path: &str, old: Document) -> Opened {
    from_parts(path, old, &working(dir, path))
}

/// The same, from two sides already in hand.
pub fn from_text(path: &str, before: &str, after: &str) -> Opened {
    from_parts(path, Document::new(path, before), after)
}

fn from_parts(path: &str, old: Document, after: &str) -> Opened {
    let new = Document::new(path, after);
    let long = old.bytes().max(new.bytes()) > MAX_SHOWN_BYTES;
    let rows = match long {
        true => Vec::new(),
        false => align(&old, &new, CONTEXT),
    };
    Opened {
        path: path.to_string(),
        marks: marks(&rows),
        words: by_line(&rows, &words(&rows, &old, &new)),
        old,
        new: Buffer::new(new),
        rows,
        long,
    }
}

/// What the rows and the colours become once the buffer has been edited.
pub struct Derived {
    pub settled: Settled,
    pub aligned: Aligned,
}

/// Aligns the file again, and colours `new` from the tree it carries.
pub fn derived(path: &str, old: &Document, new: Document) -> Derived {
    let rows = align(old, &new, CONTEXT);
    let aligned = Aligned {
        lines: rows.iter().map(|row| text_of(old, &new, row)).collect(),
        marks: marks(&rows),
        words: words(&rows, old, &new),
        path: path.to_string(),
        indent: new.indent().width(),
        long: false,
        rows,
    };
    Derived {
        aligned,
        settled: new.settled(),
    }
}

/// The file in HEAD, or nothing when it was never committed.
async fn committed(dir: &Path, path: &str) -> String {
    Git::at(dir)
        .show("HEAD", path)
        .await
        .unwrap_or_else(|_| String::new())
}

/// The file on disk, or nothing when it is gone or unreadable.
fn working(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap_or_default()
}
