//! The selected worktree a read was asked for.

use groove_types::DiffMode;

use crate::AppState;

/// What a read of the selected worktree is for; an answer that lands after it moved is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asked {
    pub worktree: groove_types::WorktreeId,
    pub dir: std::path::PathBuf,
    pub mode: DiffMode,
    pub base: Option<String>,
    pub commit: Option<String>,
}

impl Asked {
    pub(crate) fn now(state: &AppState) -> Option<Self> {
        let one = state.session.selected_worktree()?;
        Some(Self {
            worktree: one.id.clone(),
            dir: one.dir(),
            mode: state.workspace.mode,
            base: one.base_ref.clone(),
            commit: state.workspace.commit.as_ref().map(|at| at.sha.clone()),
        })
    }

    /// Whether the selection is still the one this was read for.
    pub(crate) fn holds(&self, state: &AppState) -> bool {
        Self::now(state).as_ref() == Some(self)
    }

    /// The same, whatever commit the surface shows meanwhile.
    pub(crate) fn holds_place(&self, state: &AppState) -> bool {
        Self::now(state).is_some_and(|now| {
            Self {
                commit: None,
                ..now
            } == Self {
                commit: None,
                ..self.clone()
            }
        })
    }
}

pub fn stale(state: &AppState) -> bool {
    match state.session.selected_worktree().map(|one| one.id.clone()) {
        Some(worktree) => !state.workspace.holds(&worktree),
        None => false,
    }
}
