//! One commit as a change: the files it touched, and both sides of each.

use std::collections::HashMap;
use std::path::Path;

use groove_git::Git;
use groove_types::{CommitEntry, FileDiff, FileStatus, Result};

use crate::changes::{Changes, aligned};
use crate::opened::{Opened, from_text};

/// The newest commits of the branch, the base's own marked as its.
pub async fn commits(dir: &Path, base: Option<&str>, limit: usize) -> Result<Vec<CommitEntry>> {
    Ok(Git::at(dir).log(base, limit).await?)
}

/// What one commit changed: its files, and their rows from the two sides it has.
pub async fn at_commit(dir: &Path, sha: &str) -> Result<(Vec<FileDiff>, Changes)> {
    let git = Git::at(dir);
    let counts = git.changed_in(sha).await?;
    let paths: Vec<String> = counts.iter().map(|one| one.path.clone()).collect();
    let (before, after) = git.sides_in(sha, &paths).await?;
    let files = counts
        .iter()
        .map(|one| FileDiff {
            path: one.path.clone(),
            added: one.added.unwrap_or(0),
            deleted: one.deleted.unwrap_or(0),
            status: status_of(&one.path, &before, &after),
            staged: None,
        })
        .collect();
    let rows = paths
        .iter()
        .map(|path| {
            let old = side(&before, path);
            let new = side(&after, path);
            aligned(path, old, new)
        })
        .collect();
    Ok((files, Changes::new(rows)))
}

/// One file as a commit left it, against the side its parent had.
pub async fn opened_at(dir: &Path, sha: &str, path: &str) -> Result<Opened> {
    let git = Git::at(dir);
    let after = git.show(sha, path).await.unwrap_or_default();
    let parent = format!("{sha}^");
    let before = match git.ref_exists(&parent).await? {
        true => git.show(&parent, path).await.unwrap_or_default(),
        false => String::new(),
    };
    Ok(from_text(path, &before, &after))
}

fn side<'a>(held: &'a HashMap<String, String>, path: &str) -> &'a str {
    held.get(path).map(String::as_str).unwrap_or_default()
}

/// What the commit did to a path: the side it is missing from says which.
fn status_of(
    path: &str,
    before: &HashMap<String, String>,
    after: &HashMap<String, String>,
) -> FileStatus {
    match (before.contains_key(path), after.contains_key(path)) {
        (false, true) => FileStatus::Added,
        (true, false) => FileStatus::Deleted,
        _ => FileStatus::Modified,
    }
}
