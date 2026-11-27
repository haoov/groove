//! The paths of a worktree, walked by its ignore rules.

use std::path::Path;

use groove_types::{Error, Result};
use ignore::WalkBuilder;

#[cfg(test)]
mod tests;

/// A walk of the worktree at `dir` that keeps hidden files, skips `.git` and keeps to the ignore rules.
pub fn walker(dir: &Path) -> WalkBuilder {
    let mut walk = WalkBuilder::new(dir);
    walk.hidden(false)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != ".git");
    walk
}

/// Every file under `dir` the ignore rules leave, at most `cap`, from the root.
pub async fn paths(dir: &Path, cap: usize) -> Result<Vec<String>> {
    let dir = dir.to_path_buf();
    off(move || listed(&dir, cap)).await
}

async fn off<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| Error::internal(format!("the walk failed: {e}")))
}

pub(crate) fn listed(dir: &Path, cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    let walk = walker(dir).build();
    for entry in walk.flatten() {
        if out.len() >= cap {
            break;
        }
        if !entry.metadata().is_ok_and(|meta| meta.is_file()) {
            continue;
        }
        if let Ok(path) = entry.path().strip_prefix(dir) {
            out.push(path.to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}
