//! What the session capability asks of the store and the pool.

mod log;
mod promote;
mod repos;

use std::path::Path;

use groove_sessions::Store;
use groove_timeline::Timeline;
use groove_types::{
    Error, Repo, Session, SessionId, SessionState, Task, Timestamp, Worktree, WorktreeId,
    WorktreeSpec, WorktreeStatus,
};
use groove_worktree::Pool;

use crate::Living;

/// What a session holds, as recorded and as git reports it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contents {
    pub repos: Vec<Repo>,
    pub worktrees: Vec<Worktree>,
    pub status: std::collections::BTreeMap<WorktreeId, WorktreeStatus>,
    /// The files marked read, by the worktree they belong to.
    pub read: Vec<(WorktreeId, String)>,
}

/// A worktree just made, with what the user should hear about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub repo: Repo,
    pub worktree: Worktree,
    pub notes: Vec<String>,
}

/// The session capability's module handles, cheap to clone into a job.
#[derive(Clone)]
pub struct Service {
    store: Store,
    pool: Pool,
    timeline: Timeline,
}

impl Service {
    pub fn new(store: Store, pool: Pool) -> Self {
        let timeline = Timeline::new(store.db().clone());
        Self {
            store,
            pool,
            timeline,
        }
    }

    /// The session rows, whose database the other stores share.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// On a private in-memory database, the pool under `root`, for tests up the stack.
    pub async fn in_memory(root: &Path) -> Result<Self, Error> {
        let store = Store::in_memory().await?;
        let pool = Pool::new(store.db().clone(), root);
        Ok(Self::new(store, pool))
    }

    /// Inserts the task's session, the task beside it, and puts it on the rail.
    pub async fn create_task(
        &self,
        session: &Session,
        task: &Task,
        now: Timestamp,
    ) -> Result<(), Error> {
        self.store.create_task(session, task).await?;
        self.on_rail(&session.id, now).await
    }

    /// Inserts the review's session, puts it on the rail, then checks out the MR's branch.
    pub async fn open_review(
        &self,
        session: &Session,
        name: &str,
        spec: &WorktreeSpec,
        now: Timestamp,
    ) -> Result<Added, Error> {
        self.store.create_review(session).await?;
        self.on_rail(&session.id, now).await?;
        self.add_repo(session, name, spec, None).await
    }

    /// Inserts the explorer and puts it on the rail.
    pub async fn create_explorer(&self, session: &Session, now: Timestamp) -> Result<(), Error> {
        self.store.create_explorer(session).await?;
        self.on_rail(&session.id, now).await
    }

    async fn on_rail(&self, id: &SessionId, now: Timestamp) -> Result<(), Error> {
        self.set_opened(id, Some(now)).await?;
        Ok(self.store.set_seen(id, now).await?)
    }

    pub async fn rename_explorer(&self, id: &SessionId, title: &str) -> Result<(), Error> {
        Ok(self.store.rename_explorer(id, title).await?)
    }

    /// The session gone: its worktrees, its row. Unforced, work not yet landed stops it.
    pub async fn remove(&self, id: &SessionId, force: bool) -> Result<(), Error> {
        self.pool.cleanup_session(id, force).await?;
        Ok(self.store.remove(id).await?)
    }

    /// On the rail with its directory made, or off it.
    pub async fn set_opened(&self, id: &SessionId, at: Option<Timestamp>) -> Result<(), Error> {
        if at.is_some() {
            self.pool.session_dir(id)?;
        }
        Ok(self.store.set_opened(id, at).await?)
    }

    pub async fn set_seen(&self, id: &SessionId, at: Timestamp) -> Result<(), Error> {
        Ok(self.store.set_seen(id, at).await?)
    }

    pub async fn set_selected_worktree(
        &self,
        id: &SessionId,
        worktree: Option<&WorktreeId>,
    ) -> Result<(), Error> {
        Ok(self.store.set_selected_worktree(id, worktree).await?)
    }

    pub async fn set_auto_approve(&self, id: &SessionId, on: bool) -> Result<(), Error> {
        Ok(self.store.set_auto_approve(id, on).await?)
    }

    /// Every session on disk, with its repos and worktrees.
    pub async fn living(&self) -> Result<Vec<Living>, Error> {
        let mut out = Vec::new();
        for (session, _) in self.store.living().await? {
            let worktrees = self.pool.worktrees_of(&session.id).await?;
            let repos = self.store.repos_of(&session.id).await?.len();
            out.push(Living {
                session,
                worktrees,
                repos,
            });
        }
        Ok(out)
    }

    /// The rail as it was: every session with an `opened_at`, in that order.
    pub async fn opened(&self) -> Result<Vec<(Session, SessionState)>, Error> {
        Ok(self.store.opened().await?)
    }

    /// One file of a worktree marked read, or the mark taken off it.
    pub async fn set_read(
        &self,
        id: &SessionId,
        worktree: &WorktreeId,
        path: &str,
        read: bool,
    ) -> Result<(), Error> {
        Ok(self.store.set_read(id, worktree, path, read).await?)
    }

    pub async fn contents(&self, id: &SessionId) -> Result<Contents, Error> {
        let mut repos = Vec::new();
        for repo_id in self.store.repos_of(id).await? {
            repos.push(self.pool.repo(&repo_id).await?);
        }
        let worktrees = self.pool.worktrees_of(id).await?;
        let mut status = std::collections::BTreeMap::new();
        for worktree in &worktrees {
            let told = self.pool.status(worktree).await.unwrap_or_default();
            status.insert(worktree.id.clone(), told);
        }
        Ok(Contents {
            read: self.store.reads_of(id).await?,
            repos,
            worktrees,
            status,
        })
    }
}
