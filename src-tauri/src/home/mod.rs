//! The Home snapshot: what is *locally real* right now.
//!
//! Home's job is the state a task source can't show — which sessions are checked out,
//! their repos/worktrees, and each MR's id + state. One SQL statement
//! (store::home::snapshot) delivers every row, so a normal load is fully local;
//! an explicit refresh (`force_mr`) additionally re-reads each MR's live state.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::db::models::SessionKind;
use crate::core::db::store::home::HomeRow;
use crate::core::db::store;

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct HomeMr {
    pub id: String,
    /// "gitlab" | "github" — the UI picks `!42` or `#42` from it.
    pub platform: String,
    pub remote_id: String,
    pub state: String,
    pub url: String,
    /// Pipeline status; None when unknown or the MR has no pipeline.
    pub ci: Option<String>,
    #[ts(type = "number")]
    pub unresolved: i64,
    /// Carries at least one approval — surfaced as a pill on Home.
    pub approved: bool,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct HomeRepo {
    pub repo_id: String,
    pub project: String,
    pub worktree_id: Option<String>,
    pub branch: Option<String>,
    /// A worktree row exists for this repo.
    pub provisioned: bool,
    /// Provisioned, but the directory is gone (deleted by hand / stale row).
    pub missing: bool,
    /// Working-tree changes (untracked files included, matching the sidebar chips).
    #[ts(type = "number")]
    pub modified: i64,
    #[ts(type = "number")]
    pub staged: i64,
    #[ts(type = "number")]
    pub conflicted: i64,
    #[ts(type = "number")]
    pub ahead: i64,
    #[ts(type = "number")]
    pub behind: i64,
    /// Line delta against the diff base (the MR target for review sessions).
    #[ts(type = "number")]
    pub added: i64,
    #[ts(type = "number")]
    pub deleted: i64,
    #[ts(type = "number")]
    pub files_changed: i64,
    pub mr: Option<HomeMr>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct HomeEntry {
    pub short_id: String,
    pub title: String,
    pub status: String,
    /// Priority as the source names it; None for synthetic sessions.
    pub priority: Option<String>,
    /// Where the task came from; None for explorer and review sessions.
    pub provider: Option<String>,
    pub external_url: Option<String>,
    pub kind: SessionKind,
    pub repos: Vec<HomeRepo>,
}

#[tauri::command]
pub async fn get_home_snapshot(
    force_mr: Option<bool>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<HomeEntry>, String> {
    snapshot(force_mr.unwrap_or(false), &pool)
        .await
        .map_err(|e| e.to_string())
}

async fn snapshot(force_mr: bool, pool: &SqlitePool) -> anyhow::Result<Vec<HomeEntry>> {
    let rows = store::home::snapshot(pool).await?;
    let entries = group_rows(rows);

    Ok(futures_util::future::join_all(
        entries.into_iter().map(|(entry, repo_rows)| async move {
            let repos = futures_util::future::join_all(
                repo_rows.into_iter().map(|row| repo_state(row, force_mr, pool)),
            )
            .await;
            HomeEntry { repos, ..entry }
        }),
    )
    .await)
}

/// Fold the flat snapshot rows into ONE entry per session, in first-seen order.
///
/// Looks the session up rather than trusting that its rows are adjacent: they are
/// only adjacent while the query's ORDER BY happens to group them, and a tie there
/// used to interleave two sessions and list one of them twice.
fn group_rows(rows: Vec<HomeRow>) -> Vec<(HomeEntry, Vec<HomeRow>)> {
    let mut entries: Vec<(HomeEntry, Vec<HomeRow>)> = vec![];
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for row in rows {
        let at = *seen.entry(row.session_id.clone()).or_insert_with(|| {
            entries.push((
                HomeEntry {
                    short_id: row.session_id.clone(),
                    title: row.title.clone(),
                    status: row.status.clone().unwrap_or_else(|| "in_progress".to_string()),
                    priority: row.priority.clone(),
                    provider: row.provider.clone(),
                    external_url: row.external_url.clone(),
                    kind: row.kind,
                    repos: vec![],
                },
                vec![],
            ));
            entries.len() - 1
        });
        if row.repo_id.is_some() {
            entries[at].1.push(row);
        }
    }
    entries
}

async fn repo_state(row: HomeRow, force_mr: bool, pool: &SqlitePool) -> HomeRepo {
    let base = HomeRepo {
        repo_id: row.repo_id.clone().unwrap_or_default(),
        project: row.project.clone().unwrap_or_default(),
        worktree_id: None,
        branch: None,
        provisioned: false,
        missing: false,
        modified: 0,
        staged: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        added: 0,
        deleted: 0,
        files_changed: 0,
        mr: None,
    };

    let (Some(worktree_id), Some(path), Some(branch)) =
        (row.worktree_id.clone(), row.worktree_path.clone(), row.branch.clone())
    else {
        return base;
    };

    // Home Live shows only repo · branch · MR id + state, so the snapshot stays
    // local: no git status/delta and no forge signals. The git-stat fields keep
    // their zero defaults; the frontend does not read them here.
    let mr = mr_for(&row, force_mr, pool).await;

    HomeRepo {
        worktree_id: Some(worktree_id),
        branch: Some(branch),
        provisioned: true,
        missing: !std::path::Path::new(&path).exists(),
        mr,
        ..base
    }
}

async fn mr_for(row: &HomeRow, force_mr: bool, pool: &SqlitePool) -> Option<HomeMr> {
    let (mr_id, platform, remote_id, url, mut state) = (
        row.mr_id.clone()?,
        row.mr_platform.clone()?,
        row.mr_remote_id.clone()?,
        row.mr_url.clone()?,
        row.mr_state.clone()?,
    );

    // Only on an explicit refresh: re-read the live state and persist it if it
    // moved (a merge leaves the stored row reading "open"). A failed fetch keeps
    // the stored value. Normal loads stay fully local.
    if force_mr {
        let repo = crate::core::db::models::Repo {
            id: row.repo_id.clone()?,
            host: row.repo_host.clone()?,
            group_path: row.repo_group_path.clone().unwrap_or_default(),
            project: row.project.clone().unwrap_or_default(),
            local_path: row.repo_local_path.clone()?,
        };
        if let Some(fresh) = crate::forge::mr_state(&repo, &remote_id).await {
            if fresh != state {
                let _ = store::mrs::set_state(pool, &mr_id, &fresh).await;
                state = fresh;
            }
        }
    }

    // Home shows only the id + state; the other signals are no longer fetched.
    Some(HomeMr {
        id: mr_id,
        platform,
        remote_id,
        state,
        url,
        ci: None,
        unresolved: 0,
        approved: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(session: &str, project: Option<&str>) -> HomeRow {
        HomeRow {
            session_id: session.to_string(),
            kind: SessionKind::Task,
            title: format!("Title of {session}"),
            status: None,
            priority: None,
            provider: None,
            external_url: None,
            repo_id: project.map(|p| format!("repo-{p}")),
            project: project.map(str::to_string),
            repo_host: None,
            repo_group_path: None,
            repo_local_path: None,
            worktree_id: None,
            branch: None,
            worktree_path: None,
            base_ref: None,
            mr_id: None,
            mr_platform: None,
            mr_remote_id: None,
            mr_url: None,
            mr_state: None,
        }
    }

    /// Two sessions opened in the same second share created_at, so the query's
    /// secondary sort used to interleave their rows — and a session whose rows
    /// were split appeared TWICE on Home.
    #[test]
    fn interleaved_rows_still_yield_one_entry_per_session() {
        let entries = group_rows(vec![
            row("B", Some("alpha")),
            row("A", Some("beta")),
            row("B", Some("charlie")),
        ]);

        let ids: Vec<&str> = entries.iter().map(|(e, _)| e.short_id.as_str()).collect();
        assert_eq!(ids, ["B", "A"], "one entry per session, in first-seen order");
        assert_eq!(entries[0].1.len(), 2, "both of B's repos land on B");
        assert_eq!(entries[1].1.len(), 1);
    }

    /// A session with no repos is still an entry, with no repo rows.
    #[test]
    fn a_session_with_no_repos_is_kept() {
        let entries = group_rows(vec![row("solo", None)]);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].1.is_empty());
    }
}
