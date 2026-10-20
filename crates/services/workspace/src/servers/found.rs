//! What the servers found in each file, merged once a publish lands so a frame only reads it.

use std::collections::BTreeMap;
use std::path::Path;

use groove_types::{Diagnostic, Severity, WorktreeId};

use super::{ServerKey, Servers};

/// One file's diagnostics, by the server that found them and merged in reading order.
#[derive(Debug, Default)]
pub(super) struct Found {
    by: BTreeMap<String, Vec<Diagnostic>>,
    merged: Vec<Diagnostic>,
    /// The most lines one diagnostic spans past its first.
    longest: usize,
    tally: (u32, u32),
}

impl Found {
    fn merge(&mut self) {
        let mut all: Vec<Diagnostic> = self.by.values().flatten().cloned().collect();
        all.sort_by_key(|one| (one.start, one.severity));
        let span = |one: &Diagnostic| one.end.line.saturating_sub(one.start.line);
        self.longest = all.iter().map(span).max().unwrap_or(0);
        let count = |severity| all.iter().filter(|one| one.severity == severity).count() as u32;
        self.tally = (count(Severity::Error), count(Severity::Warning));
        self.merged = all;
    }
}

impl Servers {
    /// What `key` said of the file at `path`, which a path outside its directory never is.
    pub fn diagnosed(&mut self, key: &ServerKey, path: &Path, found: Vec<Diagnostic>) {
        let Ok(inside) = path.strip_prefix(&key.dir) else {
            return;
        };
        let file = (key.worktree.clone(), inside.to_string_lossy().into_owned());
        let held = self.found.entry(file.clone()).or_default();
        match found.is_empty() {
            true => held.by.remove(&key.name),
            false => held.by.insert(key.name.clone(), found),
        };
        if held.by.is_empty() {
            self.found.remove(&file);
        } else {
            held.merge();
        }
    }

    /// Every server's diagnostics of one file, in reading order.
    pub fn diagnostics(&self, worktree: &WorktreeId, path: &str) -> &[Diagnostic] {
        self.held(worktree, path)
            .map_or(&[], |held| held.merged.as_slice())
    }

    /// Those that cover `line` of one file.
    pub fn diagnostics_on(&self, worktree: &WorktreeId, path: &str, line: usize) -> &[Diagnostic] {
        let Some(held) = self.held(worktree, path) else {
            return &[];
        };
        let all = &held.merged;
        let from = all.partition_point(|one| one.start.line + held.longest < line);
        let to = all.partition_point(|one| one.start.line <= line);
        &all[from..to.max(from)]
    }

    /// How many errors and warnings the servers found in one file.
    pub fn tally(&self, worktree: &WorktreeId, path: &str) -> (u32, u32) {
        self.held(worktree, path).map_or((0, 0), |held| held.tally)
    }

    /// The server stopped talking. Its diagnostics go with it.
    pub(super) fn drop_found(&mut self, key: &ServerKey) {
        for ((worktree, _), held) in self.found.iter_mut() {
            if worktree == &key.worktree && held.by.remove(&key.name).is_some() {
                held.merge();
            }
        }
        self.found.retain(|_, held| !held.by.is_empty());
    }

    fn held(&self, worktree: &WorktreeId, path: &str) -> Option<&Found> {
        self.found.get(&(worktree.clone(), path.to_string()))
    }
}
