//! What delivery knows of one worktree's MR: the row, the forge's last answer, its age.

use groove_forge::Snapshot;
use groove_types::{CiState, Mr, MrDelivery, MrFacts, MrState, MrThread};

#[derive(Debug, Default)]
pub struct Held {
    pub mr: Option<Mr>,
    pub read: Option<Snapshot>,
    /// The last read failed, or is older than the config allows; what stands is older than it looks.
    pub stale: bool,
    /// When the forge last answered for it.
    pub read_at: Option<groove_types::Timestamp>,
}

impl Held {
    pub fn is_open(&self) -> bool {
        self.state() == Some(MrState::Open)
    }

    pub fn state(&self) -> Option<MrState> {
        self.mr.as_ref().map(|mr| mr.state)
    }

    /// The MR as a worktree's row shows it.
    pub fn shown(&self) -> Option<MrDelivery> {
        let mr = self.mr.as_ref()?;
        let details = self.read.as_ref().map(|read| &read.details);
        Some(MrDelivery {
            forge: mr.forge,
            number: mr.remote_id.clone(),
            state: mr.state,
            url: mr.url.clone(),
            approved: details
                .and_then(|one| one.approval.as_ref())
                .is_some_and(|one| one.approved),
            changes_requested: details.is_some_and(|one| one.changes_requested()),
        })
    }

    /// The state of the run on its head commit, where it reported one.
    pub fn ci(&self) -> Option<CiState> {
        self.read.as_ref()?.ci.as_ref().map(|one| one.state)
    }

    pub fn threads(&self) -> &[MrThread] {
        self.read
            .as_ref()
            .map(|read| read.threads.as_slice())
            .unwrap_or_default()
    }

    /// The resolvable threads nobody has resolved.
    pub fn open_threads(&self) -> u32 {
        let open = self
            .threads()
            .iter()
            .filter(|thread| {
                let mut notes = thread.notes.iter();
                notes.any(|note| note.resolvable && !note.resolved)
            })
            .count();
        u32::try_from(open).unwrap_or(u32::MAX)
    }

    /// What the attention rules read of it, once the forge has answered.
    pub fn facts(&self) -> Option<MrFacts> {
        let (mr, read) = (self.mr.as_ref()?, self.read.as_ref()?);
        let details = &read.details;
        Some(MrFacts {
            state: Some(mr.state),
            review_requested_at: details.review_requested_at(),
            changes_requested_at: details.changes_requested_at(),
            ci: self.ci(),
            ci_finished_at: read.ci.as_ref().and_then(|one| one.finished_at),
            approved_at: details.approved_at(),
        })
    }
}
