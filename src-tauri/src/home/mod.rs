//! The Home snapshot: what is *locally real* right now.
//!
//! Home's job is the state Notion can't show — which sessions are checked out,
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
    /// Notion priority ("High"/"Medium"/"Low"); None for synthetic sessions.
    pub priority: Option<String>,
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

/// Fold the flat snapshot rows into one entry per session, keeping row order.
fn group_rows(rows: Vec<HomeRow>) -> Vec<(HomeEntry, Vec<HomeRow>)> {
    let mut entries: Vec<(HomeEntry, Vec<HomeRow>)> = vec![];
    for row in rows {
        if entries.last().map(|(e, _)| e.short_id.as_str()) != Some(row.session_id.as_str()) {
            entries.push((
                HomeEntry {
                    short_id: row.session_id.clone(),
                    title: row.title.clone(),
                    status: row.status.clone().unwrap_or_else(|| "in_progress".to_string()),
                    priority: row.priority.clone(),
                    kind: row.kind,
                    repos: vec![],
                },
                vec![],
            ));
        }
        if row.repo_id.is_some() {
            entries.last_mut().unwrap().1.push(row);
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
            group_path: String::new(),
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
