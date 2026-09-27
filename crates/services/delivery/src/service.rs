//! What delivery asks of the forge and of the MR rows.

pub(crate) mod notes;
pub(crate) mod queue;

use std::sync::Arc;

use groove_annotations::Store as Notes;
use groove_forge::{Proposed, Remote, Snapshot};
use groove_mrs::{Answered, Store};
use groove_types::{Error, Mr, MrState, Repo, Result, Worktree, WorktreeId};

pub use notes::Said;

/// One MR as the forge answered and the database now holds it.
#[derive(Debug)]
pub struct Delivered {
    pub mr: Mr,
    pub read: Snapshot,
}

impl From<Delivered> for crate::Held {
    fn from(delivered: Delivered) -> Self {
        Self {
            mr: Some(delivered.mr),
            read: Some(delivered.read),
            stale: false,
        }
    }
}

/// How the forge that serves a repo is reached.
pub type Connect = Arc<dyn Fn(&Repo) -> Result<Remote> + Send + Sync>;

/// The capability's module handles, cheap to clone into a job.
#[derive(Clone)]
pub struct Service {
    mrs: Store,
    notes: Notes,
    connect: Connect,
}

impl Service {
    /// Reaches each forge with the token its CLI holds.
    pub fn new(mrs: Store, notes: Notes) -> Self {
        Self {
            mrs,
            notes,
            connect: Arc::new(|repo| Ok(Remote::of(repo)?)),
        }
    }

    /// The same, reaching the forge another way.
    pub fn connecting(
        self,
        connect: impl Fn(&Repo) -> Result<Remote> + Send + Sync + 'static,
    ) -> Self {
        Self {
            connect: Arc::new(connect),
            ..self
        }
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

    fn remote(&self, repo: &Repo) -> Result<Remote> {
        (self.connect)(repo)
    }

    /// Every open MR the database holds, whichever worktree it belongs to.
    pub async fn open(&self) -> Result<Vec<Mr>> {
        self.mrs.open().await
    }

    /// The MR the database holds for a worktree.
    pub async fn stored(&self, worktree: &WorktreeId) -> Result<Option<Mr>> {
        self.mrs.get(worktree).await
    }

    /// An open MR read by its number, a settled one by the branch it came from.
    pub async fn read(&self, repo: &Repo, worktree: &Worktree) -> Result<Option<Delivered>> {
        let remote = self.remote(repo)?;
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
        self.kept(&remote, worktree, read).await.map(Some)
    }

    /// A new MR from the worktree's branch, written down as the forge answers.
    pub async fn open_mr(
        &self,
        repo: &Repo,
        worktree: &Worktree,
        text: &crate::Text,
    ) -> Result<Delivered> {
        let remote = self.remote(repo)?;
        let proposed = Proposed {
            head: &worktree.branch,
            base: worktree.base_ref.as_deref(),
            title: &text.title,
            body: &text.body,
        };
        let read = remote.open_mr_for(repo, proposed).await?;
        self.kept(&remote, worktree, read).await
    }

    /// The MR's title and body written again.
    pub async fn edit_mr(
        &self,
        repo: &Repo,
        worktree: &Worktree,
        text: &crate::Text,
    ) -> Result<Delivered> {
        let (remote, mr) = (self.remote(repo)?, self.held(&worktree.id).await?);
        let read = remote
            .edit_mr(repo, &mr.remote_id, &text.title, &text.body)
            .await?;
        self.kept(&remote, worktree, read).await
    }

    /// The MR closed, and the row left saying so.
    pub async fn close_mr(&self, repo: &Repo, worktree: &Worktree) -> Result<Delivered> {
        let (remote, mr) = (self.remote(repo)?, self.held(&worktree.id).await?);
        let read = remote.close_mr(repo, &mr.remote_id).await?;
        self.kept(&remote, worktree, read).await
    }

    /// The MR the worktree has, or the error that it has none.
    async fn held(&self, worktree: &WorktreeId) -> Result<Mr> {
        self.mrs
            .get(worktree)
            .await?
            .ok_or_else(|| Error::not_found(format!("{worktree} has no merge request")))
    }

    /// What the forge answered, written down.
    async fn kept(
        &self,
        remote: &Remote,
        worktree: &Worktree,
        read: Snapshot,
    ) -> Result<Delivered> {
        let answered = Answered {
            forge: remote.kind(),
            number: read.number.clone(),
            url: read.details.web_url.clone(),
            state: read.details.state,
        };
        let mr = self.mrs.save(&worktree.id, &answered).await?;
        Ok(Delivered { mr, read })
    }

    /// The rows themselves, for a test to seed.
    pub fn store(&self) -> &Store {
        &self.mrs
    }
}
