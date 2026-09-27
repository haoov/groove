//! One session on the rail: its repos, its worktrees, what git says of each, its reads.

use std::collections::{BTreeMap, BTreeSet};

use groove_types::{Repo, RepoId, Session, SessionState, Worktree, WorktreeId, WorktreeStatus};

/// One open session as the rail lists it.
#[derive(Debug)]
pub struct Open {
    pub session: Session,
    pub state: SessionState,
    pub repos: Vec<Repo>,
    pub worktrees: Vec<Worktree>,
    /// What git says of each worktree.
    pub status: BTreeMap<WorktreeId, WorktreeStatus>,
    /// The files read, per worktree, as the session remembers them.
    pub read: BTreeMap<WorktreeId, BTreeSet<String>>,
}

impl Open {
    /// Whether this file of the worktree has been marked read.
    pub fn is_read(&self, worktree: &WorktreeId, path: &str) -> bool {
        self.read
            .get(worktree)
            .is_some_and(|files| files.contains(path))
    }

    /// One file marked read, or the mark taken off it.
    pub fn mark(&mut self, worktree: &WorktreeId, path: &str, read: bool) {
        let files = self.read.entry(worktree.clone()).or_default();
        match read {
            true => files.insert(path.to_string()),
            false => files.remove(path),
        };
    }

    /// The file's mark turned over. Returns whether it is read now.
    pub fn toggle_read(&mut self, worktree: &WorktreeId, path: &str) -> bool {
        let read = !self.is_read(worktree, path);
        self.mark(worktree, path, read);
        read
    }

    /// What git says about one worktree now.
    pub fn told(&mut self, worktree: &WorktreeId, status: WorktreeStatus) {
        self.status.insert(worktree.clone(), status);
    }

    pub fn status_of(&self, worktree: &WorktreeId) -> WorktreeStatus {
        self.status.get(worktree).copied().unwrap_or_default()
    }

    pub fn selected_worktree(&self) -> Option<&Worktree> {
        let id = self.state.selected_worktree.as_ref()?;
        self.worktrees.iter().find(|w| &w.id == id)
    }

    /// The worktree, its repo if new, selected when nothing was.
    pub fn add_worktree(&mut self, repo: Repo, worktree: Worktree) {
        if !self.repos.iter().any(|r| r.id == repo.id) {
            self.repos.push(repo);
        }
        self.worktrees.retain(|w| w.id != worktree.id);
        if self.state.selected_worktree.is_none() {
            self.state.selected_worktree = Some(worktree.id.clone());
        }
        self.worktrees.push(worktree);
    }

    /// Drops the worktree; its repo goes with it when it was the last one; the selection moves.
    pub fn remove_worktree(&mut self, id: &WorktreeId) {
        let Some(at) = self.worktrees.iter().position(|w| &w.id == id) else {
            return;
        };
        let removed = self.worktrees.remove(at);
        if !self.worktrees.iter().any(|w| w.repo == removed.repo) {
            self.repos.retain(|r| r.id != removed.repo);
        }
        if self.state.selected_worktree.as_ref() == Some(id) {
            self.state.selected_worktree = self
                .worktrees
                .get(at)
                .or(self.worktrees.last())
                .map(|w| w.id.clone());
        }
    }

    pub fn remove_repo(&mut self, repo: &RepoId) {
        let ids: Vec<WorktreeId> = self
            .worktrees
            .iter()
            .filter(|w| &w.repo == repo)
            .map(|w| w.id.clone())
            .collect();
        for id in ids {
            self.remove_worktree(&id);
        }
        self.repos.retain(|r| &r.id != repo);
    }
}
