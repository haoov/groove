//! The worktree rows, read and written.

use groove_types::{Repo, RepoId, SessionId, Timestamp, Worktree, WorktreeId};

use crate::{Error, Pool, Result};

#[derive(sqlx::FromRow)]
struct RepoRow {
    id: String,
    host: String,
    group_path: String,
    project: String,
    local_path: String,
}

impl From<RepoRow> for Repo {
    fn from(r: RepoRow) -> Self {
        Repo {
            id: RepoId::new(r.id),
            host: r.host,
            group_path: r.group_path,
            project: r.project,
            local_path: r.local_path,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorktreeRow {
    id: String,
    session_id: String,
    repo_id: String,
    branch: String,
    path: String,
    base_ref: Option<String>,
    created_at: i64,
}

impl From<WorktreeRow> for Worktree {
    fn from(r: WorktreeRow) -> Self {
        Worktree {
            id: WorktreeId::new(r.id),
            session: SessionId::new(r.session_id),
            repo: RepoId::new(r.repo_id),
            branch: r.branch,
            path: r.path,
            base_ref: r.base_ref,
            created_at: Timestamp::new(r.created_at),
        }
    }
}

const WORKTREE_COLUMNS: &str = "id, session_id, repo_id, branch, path, base_ref, created_at";

impl Pool {
    pub async fn repo(&self, id: &RepoId) -> Result<Repo> {
        let row: Option<RepoRow> = sqlx::query_as(
            "SELECT id, host, group_path, project, local_path FROM repos WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(self.db.pool())
        .await?;
        row.map(Repo::from).ok_or_else(|| Error::NotFound {
            what: "repo",
            id: id.to_string(),
        })
    }

    pub(crate) async fn upsert_repo(&self, repo: &Repo) -> Result<()> {
        sqlx::query(
            "INSERT INTO repos (id, host, group_path, project, local_path) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET local_path = excluded.local_path",
        )
        .bind(repo.id.as_str())
        .bind(&repo.host)
        .bind(&repo.group_path)
        .bind(&repo.project)
        .bind(&repo.local_path)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn worktree(&self, id: &WorktreeId) -> Result<Worktree> {
        let row: Option<WorktreeRow> = sqlx::query_as(
            "SELECT id, session_id, repo_id, branch, path, base_ref, created_at FROM worktrees WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(self.db.pool())
        .await?;
        row.map(Worktree::from).ok_or_else(|| Error::NotFound {
            what: "worktree",
            id: id.to_string(),
        })
    }

    pub async fn worktrees_of(&self, session: &SessionId) -> Result<Vec<Worktree>> {
        let rows: Vec<WorktreeRow> = sqlx::query_as(
            "SELECT id, session_id, repo_id, branch, path, base_ref, created_at
             FROM worktrees WHERE session_id = ? ORDER BY created_at, id",
        )
        .bind(session.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(Worktree::from).collect())
    }

    pub(crate) async fn worktree_for_branch(
        &self,
        session: &SessionId,
        repo: &RepoId,
        branch: &str,
    ) -> Result<Option<Worktree>> {
        let row: Option<WorktreeRow> = sqlx::query_as(
            "SELECT id, session_id, repo_id, branch, path, base_ref, created_at
             FROM worktrees WHERE session_id = ? AND repo_id = ? AND branch = ?",
        )
        .bind(session.as_str())
        .bind(repo.as_str())
        .bind(branch)
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(Worktree::from))
    }

    /// One row per `(session, repo, branch)`; a second call moves the path.
    pub(crate) async fn upsert_worktree(
        &self,
        session: &SessionId,
        repo: &RepoId,
        branch: &str,
        path: &str,
        base_ref: Option<&str>,
        now: Timestamp,
    ) -> Result<Worktree> {
        sqlx::query(
            "INSERT INTO worktrees (id, session_id, repo_id, branch, path, base_ref, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(session_id, repo_id, branch) DO UPDATE SET
               path = excluded.path, base_ref = COALESCE(excluded.base_ref, worktrees.base_ref)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(session.as_str())
        .bind(repo.as_str())
        .bind(branch)
        .bind(path)
        .bind(base_ref)
        .bind(now.seconds())
        .execute(self.db.pool())
        .await?;
        self.worktree_for_branch(session, repo, branch)
            .await?
            .ok_or_else(|| Error::NotFound {
                what: "worktree",
                id: format!("{session}/{repo}@{branch}"),
            })
    }

    pub(crate) async fn remove_worktree(&self, id: &WorktreeId) -> Result<()> {
        sqlx::query("DELETE FROM worktrees WHERE id = ?")
            .bind(id.as_str())
            .execute(self.db.pool())
            .await?;
        Ok(())
    }
}

#[allow(dead_code)]
const _: &str = WORKTREE_COLUMNS;
