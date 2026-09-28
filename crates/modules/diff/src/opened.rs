//! The file being read: both its sides, and what an edit derives from them.

use std::path::Path;

use groove_git::Git;
use groove_text::{Buffer, Document, Settled, Touched};
use groove_types::Result;

use crate::changes::{Aligned, from_sides};
use crate::hunked::Hunked;

/// Above this a file is listed as changed and not shown.
pub const MAX_SHOWN_BYTES: usize = 2 << 20;

/// A file as the diff shows it: both sides, and how they line up.
#[derive(Debug)]
pub struct Opened {
    pub path: String,
    pub old: Document,
    /// The side the user edits.
    pub new: Buffer,
    pub hunked: Hunked,
    /// Too long to align; the view says so instead of drawing it.
    pub long: bool,
}

impl Opened {
    /// Takes the rows, the marks and the words of an alignment of it.
    pub fn take(&mut self, aligned: &Aligned) {
        self.hunked = aligned.hunked.clone();
    }

    /// One edit its buffer took, moved into its hunks.
    pub fn edited(&mut self, edit: Touched) {
        self.hunked
            .edited(edit, &self.old, self.new.document(), &[]);
    }
}

/// The file at `path` as it is on disk against the version in HEAD.
pub async fn opened(dir: &Path, path: &str, rev: &str) -> Result<Opened> {
    let before = committed(dir, path, rev).await;
    let after = working(dir, path);
    Ok(from_text(path, &before, &after))
}

/// The file read again, against the HEAD side already read for it.
pub fn reopened(dir: &Path, path: &str, old: Document) -> Opened {
    from_parts(path, old, &working(dir, path))
}

/// The same, from two sides already in hand.
pub fn from_text(path: &str, before: &str, after: &str) -> Opened {
    from_parts(path, Document::new(path, before), after)
}

/// The same, from two documents already read, which nothing parses again.
pub fn from_documents(path: &str, old: Document, new: Document) -> Opened {
    let aligned = from_sides(path, &old, &new, &[]);
    let mut open = Opened {
        path: path.to_string(),
        long: aligned.long,
        old,
        new: Buffer::new(new),
        hunked: Hunked::default(),
    };
    if !open.long {
        open.take(&aligned);
    }
    open
}

fn from_parts(path: &str, old: Document, after: &str) -> Opened {
    from_documents(path, old, Document::new(path, after))
}

/// What the rows and the colours become once the buffer has been edited.
pub struct Derived {
    pub settled: Settled,
    pub aligned: Aligned,
}

/// Aligns the file again, and colours `new` from the tree it carries.
pub fn derived(path: &str, old: &Document, new: Document) -> Derived {
    Derived {
        aligned: from_sides(path, old, &new, &[]),
        settled: new.settled(),
    }
}

/// The file in HEAD, or nothing when it was never committed.
async fn committed(dir: &Path, path: &str, rev: &str) -> String {
    Git::at(dir)
        .show(rev, path)
        .await
        .unwrap_or_else(|_| String::new())
}

/// The file on disk, or nothing when it is gone or unreadable.
fn working(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap_or_default()
}
