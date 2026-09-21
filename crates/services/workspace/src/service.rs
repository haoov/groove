//! What the workspace capability asks of the forge and of the MR rows.

use groove_forge::{Remote, Snapshot, Store};
use groove_types::{CiState, Mr, MrDelivery, Repo, Result, Worktree, WorktreeId};

/// One MR as the forge answered and the database now holds it.
#[derive(Debug)]
pub struct Delivered {
    pub mr: Mr,
    pub read: Snapshot,
}

impl Delivered {
    /// The MR part of the worktree's row.
    pub fn shown(&self) -> MrDelivery {
        MrDelivery {
            forge: self.mr.forge,
            number: self.mr.remote_id.clone(),
            state: self.mr.state,
            url: self.mr.url.clone(),
            approved: self
                .read
                .details
                .approval
                .as_ref()
                .is_some_and(|one| one.approved),
            changes_requested: self.read.details.changes_requested(),
        }
    }

    /// The state of the run on its head commit, where it reported one.
    pub fn ci(&self) -> Option<CiState> {
        self.read.ci.as_ref().map(|one| one.state)
    }

    /// The threads nobody has resolved.
    pub fn notes(&self) -> u32 {
        let open = self
            .read
            .threads
            .iter()
            .filter(|thread| thread.notes.iter().any(|note| !note.resolved))
            .count();
        u32::try_from(open).unwrap_or(u32::MAX)
    }
}

/// The workspace capability's module handles, cheap to clone into a job.
#[derive(Clone)]
pub struct Service {
    mrs: Store,
}

impl Service {
    pub fn new(mrs: Store) -> Self {
        Self { mrs }
    }

    /// On a private in-memory database, for tests of the service itself.
    pub async fn in_memory() -> Result<Self> {
        Ok(Self::new(Store::in_memory().await?))
    }

    /// On the database the sessions live in, where the worktrees an MR hangs off are.
    pub fn beside(sessions: &groove_sessions::Store) -> Self {
        Self::new(Store::new(sessions.db().clone()))
    }

    /// Every open MR the database holds, whichever worktree it belongs to.
    pub async fn open(&self) -> Result<Vec<Mr>> {
        Ok(self.mrs.open().await?)
    }

    /// The MR the database holds for a worktree.
    pub async fn stored(&self, worktree: &WorktreeId) -> Result<Option<Mr>> {
        Ok(self.mrs.get(worktree).await?)
    }

    /// Whether Groove can read the forge a host carries.
    pub fn reads(host: &str) -> bool {
        Remote::reads(host)
    }

    /// The forge that serves a repo, called with the token its CLI holds.
    pub fn remote(repo: &Repo) -> Result<Remote> {
        Ok(Remote::of(repo)?)
    }

    /// The worktree's MR read again: by the number already known, so a merge is seen,
    /// and by the branch until one is.
    pub async fn read(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &Worktree,
    ) -> Result<Option<Delivered>> {
        let read = match self.mrs.get(&worktree.id).await? {
            Some(mr) => Some(remote.read_mr(repo, &mr.remote_id).await?),
            None => remote.open_mr(repo, &worktree.branch).await?,
        };
        let Some(read) = read else {
            self.mrs.remove(&worktree.id).await?;
            return Ok(None);
        };
        let mr = self.mrs.save(&worktree.id, remote.kind(), &read).await?;
        Ok(Some(Delivered { mr, read }))
    }

    /// Forgets a worktree's MR, for one the user closed.
    pub async fn forget(&self, worktree: &WorktreeId) -> Result<()> {
        Ok(self.mrs.remove(worktree).await?)
    }

    /// The rows themselves, for a test to seed.
    pub fn store(&self) -> &Store {
        &self.mrs
    }
}
