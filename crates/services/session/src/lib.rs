//! The session capability. Its slice of `AppState`, the operations on it, its events.

mod service;

use groove_types::{
    Session, SessionId, SessionKind, SessionState, Timestamp, Worktree, WorktreeDelivery,
    WorktreeId,
};
pub use service::Service;

/// One open session as the rail lists it.
#[derive(Debug)]
pub struct Open {
    pub session: Session,
    pub state: SessionState,
    pub worktrees: Vec<Worktree>,
    pub delivery: Vec<(WorktreeId, WorktreeDelivery)>,
}

/// The `session` slice of `AppState`: the open sessions in the order opened.
#[derive(Debug, Default)]
pub struct State {
    pub open: Vec<Open>,
    pub selected: Option<SessionId>,
}

impl State {
    pub fn get(&self, id: &SessionId) -> Option<&Open> {
        self.open.iter().find(|o| &o.session.id == id)
    }

    pub fn selected(&self) -> Option<&Open> {
        self.get(self.selected.as_ref()?)
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
            worktrees: Vec::new(),
            delivery: Vec::new(),
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
            worktrees: Vec::new(),
            delivery: Vec::new(),
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
