//! What a worktree row shows of its delivery: the MR, the CI, the counts.

use crate::{CiState, Forge, MrState, ReviewState, WorktreeStatus};

/// The MR part of a worktree row.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct MrDelivery {
    pub forge: Forge,
    /// What the forge calls it, without the sigil.
    pub number: String,
    pub state: MrState,
    pub url: String,
    pub review: Option<ReviewState>,
}

impl MrDelivery {
    /// What its forge calls it, sigil and number.
    pub fn named(&self) -> String {
        format!("{}{}", self.forge.sigil(), self.number)
    }
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
    /// An MR of its own still open.
    pub fn is_open(&self) -> bool {
        self.mr.as_ref().is_some_and(|mr| mr.state == MrState::Open)
    }
}
