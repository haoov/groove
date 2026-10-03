//! What changed in a worktree, as counts per file.

use std::path::Path;

use groove_git::parse::unquote_path;
use groove_git::{Change, Counts, Git};
use groove_types::{FileDiff, FileStatus, Result};

/// Every changed file of the worktree against HEAD, untracked files included, by path.
pub async fn summary(dir: &Path) -> Result<Vec<FileDiff>> {
    let git = Git::at(dir);
    let changes = git.status().await?;
    let counts = match git.ref_exists("HEAD").await? {
        true => git.numstat("HEAD").await?,
        false => Vec::new(),
    };
    let mut files: Vec<FileDiff> = changes
        .iter()
        .map(|change| file(change, &counts, dir))
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn file(change: &Change, counts: &[Counts], dir: &Path) -> FileDiff {
    let path = unquote_path(&change.path);
    let (added, deleted) = match change.is_untracked() {
        true => whole_file(dir, &path),
        false => counted(counts, &path),
    };
    FileDiff {
        added: added.unwrap_or(0),
        deleted: deleted.unwrap_or(0),
        status: status_of(change),
        staged: Some(change.is_staged()),
        path,
    }
}

/// The index letter for a staged file, the worktree letter for the rest.
fn status_of(change: &Change) -> FileStatus {
    let letter = match change.is_staged() {
        true => change.x,
        false => change.y,
    };
    match letter {
        '?' => FileStatus::Untracked,
        'A' => FileStatus::Added,
        'D' => FileStatus::Deleted,
        'R' | 'C' => FileStatus::Renamed,
        _ => FileStatus::Modified,
    }
}

fn counted(counts: &[Counts], path: &str) -> (Option<u32>, Option<u32>) {
    match counts.iter().find(|row| row.path == path) {
        Some(row) => (row.added, row.deleted),
        None => (None, None),
    }
}

/// All additions, or nothing at all for a file too big to read.
fn whole_file(dir: &Path, path: &str) -> (Option<u32>, Option<u32>) {
    let full = dir.join(path);
    let Ok(meta) = std::fs::metadata(&full) else {
        return (None, None);
    };
    if !meta.is_file() || meta.len() > groove_types::TEXT_MAX_BYTES {
        return (None, None);
    }
    match std::fs::read_to_string(&full) {
        Ok(text) => (Some(text.lines().count() as u32), Some(0)),
        Err(_) => (None, None),
    }
}

/// Every file the branch changed since `rev`, committed, staged or untracked.
pub async fn summary_against(dir: &Path, rev: &str) -> Result<Vec<FileDiff>> {
    let git = Git::at(dir);
    let counts = git.numstat(rev).await?;
    let paths: Vec<String> = counts.iter().map(|one| one.path.clone()).collect();
    let before = git.blobs(rev, &paths).await?;
    let now = git.status().await?;
    let staged = staged_of(&now);
    let mut files: Vec<FileDiff> = counts
        .iter()
        .map(|one| against(one, dir, &before, &staged))
        .collect();
    files.extend(untracked(&now, dir));
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// One file of that change: its counts, and what it became.
fn against(
    counts: &Counts,
    dir: &Path,
    before: &std::collections::HashMap<String, String>,
    staged: &std::collections::HashMap<String, bool>,
) -> FileDiff {
    let path = unquote_path(&counts.path);
    let stands = dir.join(&path).exists();
    FileDiff {
        added: counts.added.unwrap_or(0),
        deleted: counts.deleted.unwrap_or(0),
        status: FileStatus::of(before.contains_key(&path), stands),
        staged: staged.get(&path).copied(),
        path,
    }
}

/// What the index holds, for the files it holds anything of.
fn staged_of(changes: &[Change]) -> std::collections::HashMap<String, bool> {
    changes
        .iter()
        .map(|change| (unquote_path(&change.path), change.is_staged()))
        .collect()
}

/// The files git does not track, which no rev can be compared against.
fn untracked(changes: &[Change], dir: &Path) -> Vec<FileDiff> {
    changes
        .iter()
        .filter(|change| change.is_untracked())
        .map(|change| file(change, &[], dir))
        .collect()
}
