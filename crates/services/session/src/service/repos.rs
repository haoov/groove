//! The session's repos and worktrees: attached, cut, closed, and what git says of them.

use groove_types::{
    Error, PoolEntry, Repo, RepoId, Session, SessionId, Timestamp, Worktree, WorktreeId,
    WorktreeSpec, WorktreeStatus,
};
use groove_worktree::Pool;

use super::{Added, Service};

impl Service {
    /// A pool clone, or a new clone of a URL, attached to the session with its first worktree.
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
        self.provision(session, repo, spec, tag).await
    }

    async fn provision(
        &self,
        session: &Session,
        repo: Repo,
        spec: &WorktreeSpec,
        tag: Option<&str>,
    ) -> Result<Added, Error> {
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
        self.provision(session, repo, spec, tag).await
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
