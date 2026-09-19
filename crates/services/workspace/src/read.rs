//! What the worktree is read for: the summary, the alignments, the documents, a search.

use std::path::Path;

use groove_types::{FileDiff, Result};

use crate::{Changes, Derived, Document, Found, Opened, Painted, Search};

pub async fn summary(dir: &Path) -> Result<Vec<FileDiff>> {
    groove_diff::summary(dir).await
}

/// Every changed file aligned, with no document held.
pub async fn changes(dir: &Path, files: &[FileDiff]) -> Changes {
    groove_diff::changes(dir, files).await
}

/// Every line under the worktree holding `query`, in batches as they are found.
pub fn grep(
    dir: &Path,
    query: &str,
    under: &str,
    search: std::sync::Arc<Search>,
    found: impl FnMut(Vec<Found>) + Send,
) {
    groove_grep::walk(dir, query, under, FOUND_MAX, search, found);
}

/// How many matches a search across the worktree keeps.
pub const FOUND_MAX: usize = 500;

/// Both sides of these paths, parsed, read in one git process.
pub async fn painted(dir: &Path, paths: Vec<String>) -> Vec<(String, Painted)> {
    let heads = groove_git::Git::at(dir)
        .blobs("HEAD", &paths)
        .await
        .unwrap_or_default();
    paths
        .into_iter()
        .map(|path| {
            let before = heads.get(&path).map(String::as_str).unwrap_or_default();
            let after = std::fs::read_to_string(dir.join(&path)).unwrap_or_default();
            let painted = Painted {
                old: Document::new(&path, before),
                new: Document::new(&path, &after),
            };
            (path, painted)
        })
        .collect()
}

/// One file of the worktree, both sides and the rows between them.
pub async fn opened(dir: &Path, path: &str) -> Result<Opened> {
    groove_diff::opened(dir, path).await
}

/// The same file, against the HEAD side already read for it.
pub fn reopened(dir: &Path, path: &str, old: Document) -> Opened {
    groove_diff::reopened(dir, path, old)
}

/// The colours and the alignment of the document the buffer now holds.
pub fn derived(path: &str, old: &Document, new: Document) -> Derived {
    groove_diff::derived(path, old, new)
}
