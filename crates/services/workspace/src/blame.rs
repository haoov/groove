//! Who last changed each line of a file, kept for the read it was taken from.

use std::collections::BTreeMap;
use std::path::Path;

use groove_types::{BlameLine, CommitEntry, Result, WorktreeId};

/// Which read of a file a blame belongs to: the workspace's stamp, and the buffer's revision.
pub type Read = (u64, u64);

/// The blames of one worktree's files, and the one a read is out for.
#[derive(Debug, Default)]
pub struct Blames {
    worktree: Option<WorktreeId>,
    files: BTreeMap<String, (Read, Vec<BlameLine>)>,
    asking: Option<(String, Read)>,
    /// The last read that failed, not asked again.
    failed: Option<(String, Read)>,
}

impl Blames {
    /// The lines of `path`, when they were read from this read of it.
    pub fn of(&self, worktree: &WorktreeId, path: &str, read: Read) -> Option<&[BlameLine]> {
        if self.worktree.as_ref() != Some(worktree) {
            return None;
        }
        let (at, lines) = self.files.get(path)?;
        (*at == read).then_some(lines.as_slice())
    }

    /// Whether a read of `path` is owed: none kept for this read, and none out for it.
    pub fn owed(&self, worktree: &WorktreeId, path: &str, read: Read) -> bool {
        let this = Some((path.to_string(), read));
        let out = self.asking == this || self.failed == this;
        !out && self.of(worktree, path, read).is_none()
    }

    pub fn asked(&mut self, worktree: &WorktreeId, path: &str, read: Read) {
        self.into(worktree);
        self.asking = Some((path.to_string(), read));
    }

    /// The lines read for `path`, kept unless the worktree moved meanwhile.
    pub fn took(&mut self, worktree: &WorktreeId, path: String, read: Read, lines: Vec<BlameLine>) {
        if self.worktree.as_ref() != Some(worktree) {
            return;
        }
        if self.asking.as_ref().is_some_and(|(at, _)| *at == path) {
            self.asking = None;
        }
        self.files.insert(path, (read, lines));
    }

    pub fn failed(&mut self, path: &str, read: Read) {
        if self.asking.as_ref().is_some_and(|(at, _)| at == path) {
            self.asking = None;
        }
        self.failed = Some((path.to_string(), read));
    }

    /// A commit some kept line was last changed in, as the history lists it.
    pub fn entry(&self, sha: &str) -> Option<CommitEntry> {
        let line = self
            .files
            .values()
            .flat_map(|(_, lines)| lines)
            .find(|one| one.sha == sha)?;
        Some(CommitEntry {
            sha: line.sha.clone(),
            short_sha: line.short_sha.clone(),
            message: line.summary.clone(),
            author: line.author.clone(),
            at: line.at,
            is_base: false,
        })
    }

    fn into(&mut self, worktree: &WorktreeId) {
        if self.worktree.as_ref() != Some(worktree) {
            *self = Self {
                worktree: Some(worktree.clone()),
                ..Self::default()
            };
        }
    }
}

/// The blame of `path`, from `contents` when the buffer holds edits of its own.
pub async fn blame(dir: &Path, path: &str, contents: Option<String>) -> Result<Vec<BlameLine>> {
    Ok(groove_git::Git::at(dir).blame(path, contents).await?)
}
