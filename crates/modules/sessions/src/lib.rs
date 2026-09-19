//! Sessions on disk: the `sessions` row, its `session_state` leaf, its repos.

mod error;
mod rows;
mod state;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
use groove_db::Db;
use groove_types::{RepoId, Session, SessionId, SessionKind, Timestamp, WorktreeId};

use rows::SessionRow;

/// The store, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Store {
    db: Db,
}

impl Store {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// A private in-memory database with the schema, for tests up the stack.
    pub async fn in_memory() -> Result<Self> {
        Ok(Self::new(Db::in_memory().await?))
    }

    /// The pool this store writes to, for the modules that share it.
    pub fn db(&self) -> &Db {
        &self.db
    }

    pub async fn get(&self, id: &SessionId) -> Result<Option<Session>> {
        let row: Option<SessionRow> = sqlx::query_as(
            "SELECT id, kind, title, external_id, review_project, review_iid, created_at
             FROM sessions WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(Session::from))
    }

    /// Inserts an explorer. A task or review session is created by its own path.
    pub async fn create_explorer(&self, session: &Session) -> Result<()> {
        if !matches!(session.kind, SessionKind::Explorer) {
            return Err(Error::NotExplorer(session.id.clone()));
        }
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, created_at) VALUES (?, 'explorer', ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.title)
        .bind(session.created_at.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn rename_explorer(&self, id: &SessionId, title: &str) -> Result<()> {
        let done = sqlx::query("UPDATE sessions SET title = ? WHERE id = ? AND kind = 'explorer'")
            .bind(title)
            .bind(id.as_str())
            .execute(self.db.pool())
            .await?;
        found(done.rows_affected(), "explorer", id)
    }

    /// Deletes the session and everything that references it.
    pub async fn remove(&self, id: &SessionId) -> Result<()> {
        let done = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.as_str())
            .execute(self.db.pool())
            .await?;
        found(done.rows_affected(), "session", id)
    }

    pub async fn attach_repo(&self, id: &SessionId, repo: &RepoId, now: Timestamp) -> Result<()> {
        sqlx::query(
            "INSERT INTO session_repos (session_id, repo_id, added_at) VALUES (?, ?, ?)
             ON CONFLICT(session_id, repo_id) DO NOTHING",
        )
        .bind(id.as_str())
        .bind(repo.as_str())
        .bind(now.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn detach_repo(&self, id: &SessionId, repo: &RepoId) -> Result<()> {
        sqlx::query("DELETE FROM session_repos WHERE session_id = ? AND repo_id = ?")
            .bind(id.as_str())
            .bind(repo.as_str())
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Every file this session has marked read, by the worktree it belongs to.
    pub async fn reads_of(&self, id: &SessionId) -> Result<Vec<(WorktreeId, String)>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT worktree_id, path FROM read_files WHERE session_id = ? ORDER BY path",
        )
        .bind(id.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(worktree, path)| (WorktreeId::new(worktree), path))
            .collect())
    }

    /// One file marked read, or the mark taken off it.
    pub async fn set_read(
        &self,
        id: &SessionId,
        worktree: &WorktreeId,
        path: &str,
        read: bool,
    ) -> Result<()> {
        let query = match read {
            true => sqlx::query(
                "INSERT OR IGNORE INTO read_files (session_id, worktree_id, path)
                 VALUES (?, ?, ?)",
            ),
            false => sqlx::query(
                "DELETE FROM read_files WHERE session_id = ? AND worktree_id = ? AND path = ?",
            ),
        };
        query
            .bind(id.as_str())
            .bind(worktree.as_str())
            .bind(path)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    pub async fn repos_of(&self, id: &SessionId) -> Result<Vec<RepoId>> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT repo_id FROM session_repos WHERE session_id = ? ORDER BY added_at",
        )
        .bind(id.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(ids.into_iter().map(RepoId::new).collect())
    }
}

fn found(rows: u64, what: &'static str, id: &SessionId) -> Result<()> {
    if rows == 0 {
        return Err(Error::NotFound {
            what,
            id: id.to_string(),
        });
    }
    Ok(())
}
