use std::path::Path;

use groove_git::Git;
use groove_text::Document;
use groove_types::{Result, Row};

use crate::alignment::{CONTEXT, align};

/// Above this a file is listed as changed and not shown.
pub const MAX_SHOWN_BYTES: usize = 2 << 20;

/// A file as the diff shows it: both sides, and how they line up.
#[derive(Debug)]
pub struct Opened {
    pub path: String,
    pub old: Document,
    pub new: Document,
    pub rows: Vec<Row>,
    /// Too long to align; the view says so instead of drawing it.
    pub long: bool,
}

/// The file at `path` as it is on disk against the version in HEAD.
pub async fn opened(dir: &Path, path: &str) -> Result<Opened> {
    let before = committed(dir, path).await;
    let after = working(dir, path);
    Ok(from_text(path, &before, &after))
}

/// The same, from two sides already in hand.
pub fn from_text(path: &str, before: &str, after: &str) -> Opened {
    let long = before.len().max(after.len()) > MAX_SHOWN_BYTES;
    let (old, new) = (Document::new(path, before), Document::new(path, after));
    let rows = match long {
        true => Vec::new(),
        false => align(&old, &new, CONTEXT),
    };
    Opened {
        path: path.to_string(),
        old,
        new,
        rows,
        long,
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
