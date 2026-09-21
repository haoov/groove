//! What the workspace capability asks of the forge and of the MR rows.

use groove_annotations::{New, Store as Notes};
use groove_forge::{Proposed, Remote, Snapshot, Store};
use groove_types::{
    Annotation, AnnotationId, CiState, Error, Mr, MrDelivery, MrFacts, MrState, Repo, Result,
    ReviewMr, SessionId, Timestamp, Worktree, WorktreeId,
};

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

    /// What the attention rules read of this MR.
    pub fn facts(&self) -> MrFacts {
        let details = &self.read.details;
        MrFacts {
            state: Some(self.mr.state),
            review_requested_at: details.review_requested_at(),
            changes_requested_at: details.changes_requested_at(),
            ci: self.ci(),
            ci_finished_at: self.read.ci.as_ref().and_then(|one| one.finished_at),
            approved_at: details.approved_at(),
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
    notes: Notes,
}

impl Service {
    pub fn new(mrs: Store, notes: Notes) -> Self {
        Self { mrs, notes }
    }

    /// On a private in-memory database, for tests of the service itself.
    pub async fn in_memory() -> Result<Self> {
        let mrs = Store::in_memory().await?;
        let notes = Notes::new(mrs.db().clone());
        Ok(Self::new(mrs, notes))
    }

    /// On the database the sessions live in, where the worktrees an MR hangs off are.
    pub fn beside(sessions: &groove_sessions::Store) -> Self {
        let db = sessions.db().clone();
        Self::new(Store::new(db.clone()), Notes::new(db))
    }

    /// The notes this session left, whichever file they stand on.
    pub async fn notes(&self, session: &SessionId) -> Result<Vec<Annotation>> {
        self.notes.list(session).await
    }

    pub async fn create_note(&self, new: New, now: Timestamp) -> Result<Annotation> {
        self.notes.create(new, now).await
    }

    pub async fn update_note(&self, id: &AnnotationId, content: &str) -> Result<Annotation> {
        self.notes.update(id, content).await
    }

    pub async fn resolve_note(&self, id: &AnnotationId) -> Result<Annotation> {
        self.notes.resolve(id).await
    }

    pub async fn reopen_note(&self, id: &AnnotationId) -> Result<Annotation> {
        self.notes.reopen(id).await
    }

    pub async fn delete_note(&self, id: &AnnotationId) -> Result<()> {
        self.notes.delete(id).await
    }

    /// Every open MR the database holds, whichever worktree it belongs to.
    pub async fn open(&self) -> Result<Vec<Mr>> {
        Ok(self.mrs.open().await?)
    }

    /// The MR the database holds for a worktree.
    pub async fn stored(&self, worktree: &WorktreeId) -> Result<Option<Mr>> {
        Ok(self.mrs.get(worktree).await?)
    }

    /// Every open MR a host asks this user to review.
    pub async fn review_queue(host: &str) -> Result<Vec<ReviewMr>> {
        Ok(Remote::of_host(host)?.review_queue().await?)
    }

    /// The forge that serves a repo, called with the token its CLI holds.
    pub fn remote(repo: &Repo) -> Result<Remote> {
        Ok(Remote::of(repo)?)
    }

    /// An open MR read by its number, a settled one by the branch it came from.
    pub async fn read(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &Worktree,
    ) -> Result<Option<Delivered>> {
        let held = self.mrs.get(&worktree.id).await?;
        let read = match held.as_ref().filter(|mr| mr.state == MrState::Open) {
            Some(mr) => Some(remote.read_mr(repo, &mr.remote_id).await?),
            None => remote.open_mr(repo, &worktree.branch).await?,
        };
        let read = match (read, held) {
            (Some(read), _) => read,
            (None, Some(mr)) => remote.read_mr(repo, &mr.remote_id).await?,
            (None, None) => {
                self.mrs.remove(&worktree.id).await?;
                return Ok(None);
            }
        };
        let mr = self.mrs.save(&worktree.id, remote.kind(), &read).await?;
        Ok(Some(Delivered { mr, read }))
    }

    /// A new MR from the worktree's branch, written down as the forge answers.
    pub async fn open_mr(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &Worktree,
        text: &crate::Text,
    ) -> Result<Delivered> {
        let read = remote
            .open_mr_for(
                repo,
                Proposed {
                    head: &worktree.branch,
                    base: worktree.base_ref.as_deref(),
                    title: &text.title,
                    body: &text.body,
                },
            )
            .await?;
        self.kept(remote, worktree, read).await
    }

    /// The MR's title and body written again.
    pub async fn edit_mr(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &Worktree,
        text: &crate::Text,
    ) -> Result<Delivered> {
        let mr = self.held(&worktree.id).await?;
        let read = remote
            .edit_mr(repo, &mr.remote_id, &text.title, &text.body)
            .await?;
        self.kept(remote, worktree, read).await
    }

    /// The MR closed, and the row left saying so.
    pub async fn close_mr(
        &self,
        remote: &Remote,
        repo: &Repo,
        worktree: &Worktree,
    ) -> Result<Delivered> {
        let mr = self.held(&worktree.id).await?;
        let read = remote.close_mr(repo, &mr.remote_id).await?;
        self.kept(remote, worktree, read).await
    }

    /// The MR the worktree has, or the error that it has none.
    async fn held(&self, worktree: &WorktreeId) -> Result<Mr> {
        self.mrs
            .get(worktree)
            .await?
            .ok_or_else(|| Error::not_found(format!("{worktree} has no merge request")))
    }

    /// What a write answered, written down.
    async fn kept(
        &self,
        remote: &Remote,
        worktree: &Worktree,
        read: Snapshot,
    ) -> Result<Delivered> {
        let mr = self.mrs.save(&worktree.id, remote.kind(), &read).await?;
        Ok(Delivered { mr, read })
    }

    /// The rows themselves, for a test to seed.
    pub fn store(&self) -> &Store {
        &self.mrs
    }
}
