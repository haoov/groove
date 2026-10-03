//! Sessions on disk: the `sessions` row, its `session_state` leaf, its repos.

mod error;
mod promote;
mod rows;
mod state;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
use groove_db::Db;
pub use groove_db::Store as Stored;
use groove_types::{
    RepoId, Session, SessionId, SessionKind, StatusIntent, Task, Timestamp, WorktreeId,
};

pub use promote::Moved;
use rows::SessionRow;

/// The store, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Store {
    db: Db,
}

impl groove_db::Store for Store {
    fn new(db: Db) -> Self {
        Self { db }
    }

    fn db(&self) -> &Db {
        &self.db
    }
}

impl Store {
    pub async fn get(&self, id: &SessionId) -> Result<Option<Session>> {
        let row: Option<SessionRow> = sqlx::query_as(
            "SELECT s.id, s.kind, s.title, s.external_id, s.review_project, s.review_iid,
                    s.created_at, r.routine
             FROM sessions s LEFT JOIN routine_sessions r ON r.session_id = s.id
             WHERE s.id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(Session::from))
    }

    /// Inserts an explorer. A task or review session is created by its own path.
    pub async fn create_explorer(&self, session: &Session) -> Result<()> {
        if !matches!(session.kind, SessionKind::Explorer) {
            return Err(promote::wrong("explorer", &session.id));
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

    /// Inserts the session a standalone routine runs in, and names the routine beside it.
    pub async fn create_routine(&self, session: &Session) -> Result<()> {
        let SessionKind::Routine { routine } = &session.kind else {
            return Err(promote::wrong("routine", &session.id));
        };
        let mut tx = self.db.pool().begin().await?;
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, created_at) VALUES (?, 'explorer', ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.title)
        .bind(session.created_at.seconds())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO routine_sessions (session_id, routine) VALUES (?, ?)")
            .bind(session.id.as_str())
            .bind(routine)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The session of a task, with the task it works on beside it.
    pub async fn create_task(&self, session: &Session, task: &Task) -> Result<()> {
        let SessionKind::Task { external_id } = &session.kind else {
            return Err(promote::wrong("task", &session.id));
        };
        self.remember(task).await?;
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, external_id, created_at)
             VALUES (?, 'task', ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.title)
        .bind(external_id.as_str())
        .bind(session.created_at.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// The session that reviews someone else's MR.
    pub async fn create_review(&self, session: &Session) -> Result<()> {
        let SessionKind::Review { project, iid } = &session.kind else {
            return Err(promote::wrong("review", &session.id));
        };
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, review_project, review_iid, created_at)
             VALUES (?, 'review', ?, ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.title)
        .bind(project)
        .bind(i64::try_from(*iid).unwrap_or_default())
        .bind(session.created_at.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// The task as its source last reported it, for a restart to show.
    pub async fn remember(&self, task: &Task) -> Result<()> {
        sqlx::query(
            "INSERT INTO provider_tasks
                (external_id, short_id, title, status, priority, synced_at, provider,
                 url, board, branch_tag, intent, start_day, due_day, estimate, logged)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(external_id) DO UPDATE SET
                title = excluded.title, status = excluded.status,
                priority = excluded.priority, synced_at = excluded.synced_at,
                url = excluded.url, board = excluded.board,
                branch_tag = excluded.branch_tag, intent = excluded.intent,
                start_day = excluded.start_day, due_day = excluded.due_day,
                estimate = excluded.estimate, logged = excluded.logged",
        )
        .bind(task.external_id.as_str())
        .bind(&task.short_id)
        .bind(&task.title)
        .bind(&task.status)
        .bind(task.priority.map(|one| one.label()))
        .bind(task.synced_at.seconds())
        .bind(task.provider.as_str())
        .bind(&task.url)
        .bind(&task.project)
        .bind(&task.branch_tag)
        .bind(task.intent.map(StatusIntent::as_str))
        .bind(task.dates.start.map(|day| day.to_string()))
        .bind(task.dates.due.map(|day| day.to_string()))
        .bind(task.estimate)
        .bind(task.logged)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn rename_explorer(&self, id: &SessionId, title: &str) -> Result<()> {
        let done = sqlx::query(
            "UPDATE sessions SET title = ? WHERE id = ? AND kind = 'explorer'
             AND id NOT IN (SELECT session_id FROM routine_sessions)",
        )
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
