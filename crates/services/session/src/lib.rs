//! The session capability. Its slice of `AppState`, the operations on it, its events.

mod service;

use std::collections::{BTreeMap, BTreeSet};

use groove_types::{
    Repo, RepoId, Session, SessionId, SessionKind, SessionState, Timestamp, Worktree,
    WorktreeDelivery, WorktreeId, WorktreeStatus,
};
pub use service::{Added, Service};

/// One open session as the rail lists it.
#[derive(Debug)]
pub struct Open {
    pub session: Session,
    pub state: SessionState,
    pub repos: Vec<Repo>,
    pub worktrees: Vec<Worktree>,
    pub delivery: Vec<(WorktreeId, WorktreeDelivery)>,
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

    /// What git says about one worktree now.
    pub fn told(&mut self, worktree: &WorktreeId, status: WorktreeStatus) {
        match self.delivery.iter_mut().find(|(id, _)| id == worktree) {
            Some((_, delivery)) => delivery.status = status,
            None => self.delivery.push((
                worktree.clone(),
                WorktreeDelivery {
                    status,
                    ..WorktreeDelivery::default()
                },
            )),
        }
    }

    pub fn delivery_of(&self, worktree: &WorktreeId) -> Option<&WorktreeDelivery> {
        self.delivery
            .iter()
            .find(|(id, _)| id == worktree)
            .map(|(_, delivery)| delivery)
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

/// The `session` slice of `AppState`: the open sessions in the order opened.
#[derive(Debug, Default)]
pub struct State {
    pub open: Vec<Open>,
    pub selected: Option<SessionId>,
    /// The pool as last listed, for the pickers.
    pub pool: Vec<groove_types::PoolEntry>,
    /// Origin's heads per repo as last listed, for the pickers.
    pub branches: Vec<(RepoId, Vec<String>)>,
}

impl State {
    pub fn get(&self, id: &SessionId) -> Option<&Open> {
        self.open.iter().find(|o| &o.session.id == id)
    }

    pub fn selected(&self) -> Option<&Open> {
        self.get(self.selected.as_ref()?)
    }

    pub fn get_mut(&mut self, id: &SessionId) -> Option<&mut Open> {
        self.open.iter_mut().find(|o| &o.session.id == id)
    }

    /// Adds a row and selects it.
    pub fn open(&mut self, session: Session, now: Timestamp) {
        let id = session.id.clone();
        self.open.retain(|o| o.session.id != id);
        self.open.push(Open {
            session,
            state: SessionState {
                opened_at: Some(now),
                ..SessionState::default()
            },
            repos: Vec::new(),
            worktrees: Vec::new(),
            delivery: Vec::new(),
            read: BTreeMap::new(),
        });
        self.selected = Some(id);
    }

    pub fn select(&mut self, id: &SessionId, now: Timestamp) {
        let Some(open) = self.open.iter_mut().find(|o| &o.session.id == id) else {
            return;
        };
        open.state.seen_at = Some(now);
        self.selected = Some(id.clone());
    }

    pub fn rename(&mut self, id: &SessionId, title: &str) {
        if let Some(open) = self.open.iter_mut().find(|o| &o.session.id == id) {
            open.session.title = title.to_string();
        }
    }

    /// A row read back from disk, in the order stored; nothing selected by it.
    pub fn restore(&mut self, session: Session, state: SessionState) {
        self.open.retain(|o| o.session.id != session.id);
        self.open.push(Open {
            session,
            state,
            repos: Vec::new(),
            worktrees: Vec::new(),
            delivery: Vec::new(),
            read: BTreeMap::new(),
        });
    }

    /// Removes the row; the selection moves to the row that took its place, or the last one.
    pub fn close(&mut self, id: &SessionId) -> Option<Open> {
        let at = self.open.iter().position(|o| &o.session.id == id)?;
        let closed = self.open.remove(at);
        if self.selected.as_ref() == Some(id) {
            let next = self.open.get(at).or(self.open.last());
            self.selected = next.map(|o| o.session.id.clone());
        }
        Some(closed)
    }
}

/// An explorer: no ticket yet, a title the user gave or the default.
pub fn explorer(title: Option<&str>, now: Timestamp) -> Session {
    let short = uuid::Uuid::new_v4().simple().to_string();
    let title = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("Explorer")
        .to_string();
    Session {
        id: SessionId::new(format!("explorer-{}", &short[..8])),
        title,
        kind: SessionKind::Explorer,
        created_at: now,
    }
}

#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
