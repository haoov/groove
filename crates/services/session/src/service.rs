use std::path::Path;

use groove_sessions::Store;
use groove_types::{
    Error, PoolEntry, Repo, RepoId, Session, SessionId, SessionState, Timestamp, Worktree,
    WorktreeDelivery, WorktreeId, WorktreeSpec, WorktreeStatus,
};
use groove_worktree::Pool;

/// What a session holds, as recorded and as git reports it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contents {
    pub repos: Vec<Repo>,
    pub worktrees: Vec<Worktree>,
    pub delivery: Vec<(WorktreeId, WorktreeDelivery)>,
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
}

impl Service {
    pub fn new(store: Store, pool: Pool) -> Self {
        Self { store, pool }
    }

    /// On a private in-memory database, the pool under `root`, for tests up the stack.
    pub async fn in_memory(root: &Path) -> Result<Self, Error> {
        let store = Store::in_memory().await?;
        let pool = Pool::new(store.db().clone(), root);
        Ok(Self::new(store, pool))
    }

    /// Inserts the explorer and puts it on the rail.
    pub async fn create_explorer(&self, session: &Session, now: Timestamp) -> Result<(), Error> {
        self.store.create_explorer(session).await?;
        self.store.set_opened(&session.id, Some(now)).await?;
        self.store.set_seen(&session.id, now).await?;
        Ok(())
    }

    pub async fn rename_explorer(&self, id: &SessionId, title: &str) -> Result<(), Error> {
        Ok(self.store.rename_explorer(id, title).await?)
    }

    /// The session's worktrees off disk, then its row and everything under it.
    pub async fn remove(&self, id: &SessionId) -> Result<(), Error> {
        self.pool.cleanup_session(id).await?;
        Ok(self.store.remove(id).await?)
    }

    pub async fn set_opened(&self, id: &SessionId, at: Option<Timestamp>) -> Result<(), Error> {
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

    /// The rail as it was: every session with an `opened_at`, in that order.
    pub async fn opened(&self) -> Result<Vec<(Session, SessionState)>, Error> {
        Ok(self.store.opened().await?)
    }

    /// A session's repos and worktrees, and what git says about each worktree.
    /// A worktree whose directory is gone reports no counts rather than failing the load.
    pub async fn contents(&self, id: &SessionId) -> Result<Contents, Error> {
        let mut repos = Vec::new();
        for repo_id in self.store.repos_of(id).await? {
            repos.push(self.pool.repo(&repo_id).await?);
        }
        let worktrees = self.pool.worktrees_of(id).await?;
        let mut delivery = Vec::with_capacity(worktrees.len());
        for worktree in &worktrees {
            let status = self.pool.status(worktree).await.unwrap_or_default();
            delivery.push((
                worktree.id.clone(),
                WorktreeDelivery {
                    status,
                    ..WorktreeDelivery::default()
                },
            ));
        }
        Ok(Contents {
            repos,
            worktrees,
            delivery,
        })
    }

    /// Resolve the name in the pool, or clone a URL into it; record and attach the repo;
    /// cut its first worktree.
    pub async fn add_repo(
        &self,
        session: &Session,
        name: &str,
        spec: &WorktreeSpec,
        tag: Option<&str>,
    ) -> Result<Added, Error> {
        let repo = self.find_or_clone(name).await?;
        self.store
            .attach_repo(&session.id, &repo.id, Timestamp::now())
            .await?;
        let done = self.pool.provision(session, &repo, spec, tag).await?;
        Ok(Added {
            repo,
            worktree: done.worktree,
            notes: done.notes,
        })
    }

    /// A pool clone by name, or a fresh clone when the name is a git URL.
    async fn find_or_clone(&self, name: &str) -> Result<Repo, Error> {
        let entries = self.pool.list();
        match Pool::resolve(name, &entries) {
            Ok(entry) => Ok(self.pool.register(entry).await?),
            Err(groove_worktree::Error::UnknownRepo(_))
                if groove_git::RemoteUrl::parse(name).is_ok() =>
            {
                Ok(self.pool.clone(name).await?)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Another worktree on a repo the session already holds.
    pub async fn add_worktree(
        &self,
        session: &Session,
        repo: &RepoId,
        spec: &WorktreeSpec,
        tag: Option<&str>,
    ) -> Result<Added, Error> {
        let repo = self.pool.repo(repo).await?;
        let done = self.pool.provision(session, &repo, spec, tag).await?;
        Ok(Added {
            repo,
            worktree: done.worktree,
            notes: done.notes,
        })
    }

    /// Closes the worktree; the repo is detached when it was the last one.
    pub async fn close_worktree(&self, id: &WorktreeId, force: bool) -> Result<Worktree, Error> {
        let closed = self.pool.close(id, force).await?;
        let left = self.pool.worktrees_of(&closed.session).await?;
        if !left.iter().any(|w| w.repo == closed.repo) {
            self.store
                .detach_repo(&closed.session, &closed.repo)
                .await?;
        }
        Ok(closed)
    }

    /// Closes every worktree of the repo, then detaches it.
    pub async fn remove_repo(
        &self,
        session: &SessionId,
        repo: &RepoId,
        force: bool,
    ) -> Result<(), Error> {
        for worktree in self.pool.worktrees_of(session).await? {
            if &worktree.repo == repo {
                self.pool.close(&worktree.id, force).await?;
            }
        }
        Ok(self.store.detach_repo(session, repo).await?)
    }

    pub fn list_pool(&self) -> Vec<PoolEntry> {
        self.pool.list()
    }

    /// Origin's heads for the pickers.
    pub async fn list_branches(&self, repo: &RepoId) -> Result<Vec<String>, Error> {
        let repo = self.pool.repo(repo).await?;
        Ok(groove_git::Git::at(&repo.local_path)
            .remote_heads()
            .await
            .map_err(groove_worktree::Error::from)?)
    }

    pub async fn status(&self, worktree: &Worktree) -> Result<WorktreeStatus, Error> {
        Ok(self.pool.status(worktree).await?)
    }
}
