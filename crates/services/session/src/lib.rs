//! The session capability. Its slice of `AppState`, the operations on it, its events.

use groove_types::{Session, SessionId, SessionState, Worktree, WorktreeDelivery, WorktreeId};

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
    pub fn selected(&self) -> Option<&Open> {
        let id = self.selected.as_ref()?;
        self.open.iter().find(|o| &o.session.id == id)
    }
}

#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
