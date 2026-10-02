//! The session capability. Its slice of `AppState`, the operations on it, its events.

mod made;
mod open;
mod service;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use groove_types::{
    Repo, RepoId, Session, SessionId, SessionState, Timestamp, Worktree, WorktreeId,
};
pub use made::{explorer, review_session, routine_session, task_session};
pub use open::Open;
pub use service::{Added, Service};

/// The `session` slice of `AppState`: the open sessions in the order opened.
#[derive(Debug, Default)]
pub struct State {
    pub open: Vec<Open>,
    /// Every session on disk, for the board's Live column.
    pub living: Vec<Living>,
    pub selected: Option<SessionId>,
    /// The pool as last listed, for the pickers.
    pub pool: Vec<groove_types::PoolEntry>,
    /// Origin's heads per repo as last listed, for the pickers.
    pub branches: Vec<(RepoId, Vec<String>)>,
    /// What the opened sessions have done, newest first.
    pub feed: Vec<groove_types::TimelineEvent>,
}

/// How many lines the feed holds, over every session it reads.
pub const FEED_MAX: usize = 200;

/// A session that exists on disk, on the rail or not.
#[derive(Clone, Debug, PartialEq)]
pub struct Living {
    pub session: Session,
    pub worktrees: Vec<Worktree>,
    pub repos: usize,
}

impl State {
    pub fn get(&self, id: &SessionId) -> Option<&Open> {
        self.open.iter().find(|o| &o.session.id == id)
    }

    /// The tasks the sessions on disk are working.
    pub fn worked(&self) -> Vec<groove_types::ExternalId> {
        self.living
            .iter()
            .filter_map(|living| living.session.kind.task().cloned())
            .collect()
    }

    pub fn selected(&self) -> Option<&Open> {
        self.get(self.selected.as_ref()?)
    }

    /// The worktree the selected session has selected.
    pub fn selected_worktree(&self) -> Option<&Worktree> {
        self.selected()?.selected_worktree()
    }

    /// A worktree by id, with the session and the repo that hold it.
    pub fn find(&self, id: &WorktreeId) -> Option<(&Open, &Repo, &Worktree)> {
        self.open.iter().find_map(|open| {
            let worktree = open.worktrees.iter().find(|one| &one.id == id)?;
            let repo = open.repos.iter().find(|one| one.id == worktree.repo)?;
            Some((open, repo, worktree))
        })
    }

    /// The session on the rail that works this task.
    pub fn working(&self, task: &groove_types::ExternalId) -> Option<&Open> {
        self.open.iter().find(|open| open.session.kind.works(task))
    }

    /// Every worktree of the sessions on the rail.
    pub fn worktrees(&self) -> Vec<WorktreeId> {
        let all = self.open.iter().flat_map(|open| open.worktrees.iter());
        all.map(|one| one.id.clone()).collect()
    }

    pub fn get_mut(&mut self, id: &SessionId) -> Option<&mut Open> {
        self.open.iter_mut().find(|o| &o.session.id == id)
    }

    /// Adds a row and selects it.
    pub fn open(&mut self, session: Session, now: Timestamp) {
        let id = session.id.clone();
        let state = SessionState {
            opened_at: Some(now),
            ..SessionState::default()
        };
        self.restore(session, state);
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
            status: BTreeMap::new(),
            read: BTreeMap::new(),
        });
    }

    /// A line one of the rail's sessions just made, at the top of the feed.
    pub fn logged(&mut self, line: groove_types::TimelineEvent) {
        if self.get(&line.session).is_none() {
            return;
        }
        self.feed.insert(0, line);
        self.feed.truncate(FEED_MAX);
    }

    /// Removes the row and its lines; the selection moves to its neighbour.
    pub fn close(&mut self, id: &SessionId) -> Option<Open> {
        let at = self.open.iter().position(|o| &o.session.id == id)?;
        let closed = self.open.remove(at);
        self.feed.retain(|line| &line.session != id);
        if self.selected.as_ref() == Some(id) {
            let next = self.open.get(at).or(self.open.last());
            self.selected = next.map(|o| o.session.id.clone());
        }
        Some(closed)
    }
}

#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
