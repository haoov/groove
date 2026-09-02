use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::SessionKind;

/// One row per (session, attached repo) — sessions with no repos yield a
/// single row with the repo side NULL. Newest session first, repos by name.
#[derive(Debug, sqlx::FromRow)]
pub struct HomeRow {
    pub session_id: String,
    pub kind: SessionKind,
    pub title: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    /// "notion" | "github"; None for a session with no task behind it.
    pub provider: Option<String>,
    pub external_url: Option<String>,
    pub repo_id: Option<String>,
    pub project: Option<String>,
    pub repo_host: Option<String>,
    pub repo_group_path: Option<String>,
    pub repo_local_path: Option<String>,
    pub worktree_id: Option<String>,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
    pub base_ref: Option<String>,
    pub mr_id: Option<String>,
    pub mr_platform: Option<String>,
    pub mr_remote_id: Option<String>,
    pub mr_url: Option<String>,
    pub mr_state: Option<String>,
}

pub async fn snapshot(exec: impl SqliteExecutor<'_>) -> StoreResult<Vec<HomeRow>> {
    Ok(sqlx::query_as(
        "SELECT
           s.id AS session_id, s.kind, s.title,
           nt.status, nt.priority, nt.provider, nt.url AS external_url,
           r.id AS repo_id, r.project, r.host AS repo_host, r.group_path AS repo_group_path,
           r.local_path AS repo_local_path,
           w.id AS worktree_id, w.branch, w.path AS worktree_path, w.base_ref,
           m.id AS mr_id, m.platform AS mr_platform, m.remote_id AS mr_remote_id,
           m.url AS mr_url, m.state AS mr_state
         FROM sessions s
         LEFT JOIN provider_tasks nt ON nt.external_id = s.external_id
         LEFT JOIN session_repos sr ON sr.session_id = s.id
         LEFT JOIN repos r ON r.id = sr.repo_id
         LEFT JOIN worktrees w ON w.session_id = s.id AND w.repo_id = r.id
         LEFT JOIN mrs m ON m.id =
           (SELECT id FROM mrs WHERE worktree_id = w.id ORDER BY rowid DESC LIMIT 1)
         -- s.id breaks the created_at tie: unixepoch() is whole seconds, so two
         -- sessions opened in the same second would otherwise interleave their
         -- repo rows and the caller would see one session twice.
         ORDER BY s.created_at DESC, s.id, r.project",
    )
    .fetch_all(exec)
    .await?)
}
