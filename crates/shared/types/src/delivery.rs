//! What a worktree row shows of its delivery: the MR, the CI, the counts.

use crate::{CiState, MrState, WorktreeStatus};

/// The MR part of a worktree row.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MrDelivery {
    pub state: MrState,
    pub url: String,
    pub approved: bool,
    pub changes_requested: bool,
}

/// Everything a worktree row shows as icons and counts.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorktreeDelivery {
    pub status: WorktreeStatus,
    pub mr: Option<MrDelivery>,
    pub ci: Option<CiState>,
    pub notes: u32,
    /// The last poll failed; the facts are older than they look.
    pub stale: bool,
}

impl WorktreeDelivery {
    /// Merged or closed: nothing left to land.
    pub fn is_delivered(&self) -> bool {
        self.mr.as_ref().is_some_and(|mr| mr.state != MrState::Open)
    }

    /// An MR of its own still open.
    pub fn is_open(&self) -> bool {
        self.mr.as_ref().is_some_and(|mr| mr.state == MrState::Open)
    }
}
