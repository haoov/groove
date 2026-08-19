//! The Home snapshot: what is *locally real* right now.
//!
//! Home's job is the state Notion can't show — which repos are provisioned, what
//! the working trees look like, and where each MR stands. One SQL statement
//! (store::home::snapshot) delivers every row; the per-worktree git state is the
//! only remaining cost, issued concurrently, plus the cached MR signals.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::db::models::{SessionKind, Worktree};
use crate::core::db::store::home::HomeRow;
use crate::core::db::store;

#[derive(Debug, Serialize)]
pub struct HomeMr {
    pub id: String,
    /// "gitlab" | "github" — the UI picks `!42` or `#42` from it.
    pub platform: String,
    pub remote_id: String,
    pub state: String,
    pub url: String,
    /// Pipeline status; None when unknown or the MR has no pipeline.
    pub ci: Option<String>,
    pub unresolved: i64,
    /// Carries at least one approval — surfaced as a pill on Home.
    pub approved: bool,
}

#[derive(Debug, Serialize)]
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
    pub modified: i64,
    pub staged: i64,
    pub conflicted: i64,
    pub ahead: i64,
    pub behind: i64,
    /// Line delta against the diff base (the MR target for review sessions).
    pub added: i64,
    pub deleted: i64,
    pub files_changed: i64,
    pub mr: Option<HomeMr>,
}

#[derive(Debug, Serialize)]
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
                repo_rows.into_iter().map(|row| repo_state(row, force_mr)),
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

async fn repo_state(row: HomeRow, force_mr: bool) -> HomeRepo {
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

    let mr = mr_signals_for(&row, force_mr).await;

    if !std::path::Path::new(&path).exists() {
        return HomeRepo {
            worktree_id: Some(worktree_id),
            branch: Some(branch),
            provisioned: true,
            missing: true,
            mr,
            ..base
        };
    }

    let wt = Worktree {
        id: worktree_id.clone(),
        session_id: row.session_id.clone(),
        repo_id: base.repo_id.clone(),
        branch: branch.clone(),
        path,
        base_ref: row.base_ref.clone(),
        created_at: 0,
    };

    let (modified, staged, conflicted, ab) = working_tree_state(&wt.path).await;
    let (added, deleted, files_changed) = line_delta(&wt).await;
    // No upstream yet (never pushed) → count everything past the diff base as ahead.
    let (ahead, behind) = match ab {
        Some(ab) => ab,
        None => (commits_past_base(&wt).await, 0),
    };

    HomeRepo {
        worktree_id: Some(worktree_id),
        branch: Some(branch),
        provisioned: true,
        missing: false,
        modified,
        staged,
        conflicted,
        ahead,
        behind,
        added,
        deleted,
        files_changed,
        mr,
        ..base
    }
}

async fn mr_signals_for(row: &HomeRow, force_mr: bool) -> Option<HomeMr> {
    let (mr_id, platform, remote_id, url, state) = (
        row.mr_id.clone()?,
        row.mr_platform.clone()?,
        row.mr_remote_id.clone()?,
        row.mr_url.clone()?,
        row.mr_state.clone()?,
    );
    let repo = crate::core::db::models::Repo {
        id: row.repo_id.clone()?,
        host: row.repo_host.clone()?,
        group_path: String::new(),
        project: row.project.clone().unwrap_or_default(),
        local_path: row.repo_local_path.clone()?,
    };
    let sig = crate::mr_manager::mr_signals(&repo, &mr_id, &remote_id, force_mr).await;
    Some(HomeMr {
        id: mr_id,
        platform,
        remote_id,
        state,
        url,
        ci: sig.ci,
        unresolved: sig.unresolved,
        approved: sig.approved,
    })
}

/// One `git status --porcelain=v2 --branch` gives dirty/staged/conflicted AND the
/// ahead/behind pair (when the branch has an upstream) — v1 porcelain would need a
/// second `rev-list`. Returns `(modified, staged, conflicted, Some((ahead, behind)))`.
async fn working_tree_state(path: &str) -> (i64, i64, i64, Option<(i64, i64)>) {
    let Ok(out) = crate::core::git::output(path, &["status", "--porcelain=v2", "--branch"]).await
    else {
        return (0, 0, 0, None);
    };
    if !out.status.success() {
        return (0, 0, 0, None);
    }

    let (mut modified, mut staged, mut conflicted) = (0i64, 0i64, 0i64);
    let mut ab = None;

    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // "# branch.ab +2 -1"
            let mut it = rest.split_whitespace();
            let a = it.next().and_then(|s| s.trim_start_matches('+').parse::<i64>().ok());
            let b = it.next().and_then(|s| s.trim_start_matches('-').parse::<i64>().ok());
            if let (Some(a), Some(b)) = (a, b) {
                ab = Some((a, b));
            }
            continue;
        }
        match line.chars().next() {
            // "1 XY …" (ordinary) / "2 XY …" (rename): X = index side, Y = worktree
            // side, '.' meaning unchanged on that side.
            Some('1') | Some('2') => {
                let xy: Vec<char> = line.chars().skip(2).take(2).collect();
                if xy.first().is_some_and(|c| *c != '.') {
                    staged += 1;
                }
                if xy.get(1).is_some_and(|c| *c != '.') {
                    modified += 1;
                }
            }
            Some('u') => conflicted += 1,
            // Untracked files count as working-tree changes, like the sidebar chips.
            Some('?') => modified += 1,
            _ => {}
        }
    }

    (modified, staged, conflicted, ab)
}

/// `+added / −deleted` against the session's diff base — which for a review
/// session is the MR's target branch, not the repo default.
async fn line_delta(wt: &Worktree) -> (i64, i64, i64) {
    let Ok(base) = crate::core::git::refs::diff_base(
        &wt.path,
        &wt.branch,
        "vs-main",
        wt.base_ref.as_deref(),
    )
    .await
    else {
        return (0, 0, 0);
    };

    let Ok(out) = crate::core::git::output(
        &wt.path,
        &["diff", &base, "--numstat", "--no-renames", "--no-color"],
    )
    .await
    else {
        return (0, 0, 0);
    };

    let (mut added, mut deleted, mut files) = (0i64, 0i64, 0i64);
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if line.trim().is_empty() {
            continue;
        }
        files += 1;
        let mut parts = line.split('\t');
        // Binary files report "-\t-\t<path>" — counted as a file, no line delta.
        added += parts.next().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        deleted += parts.next().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    }
    (added, deleted, files)
}

/// Commits on the branch that aren't on its base — the "ahead" count for a branch
/// that was never pushed (so `branch.ab` is absent).
async fn commits_past_base(wt: &Worktree) -> i64 {
    let Ok(base) = crate::core::git::refs::diff_base(
        &wt.path,
        &wt.branch,
        "vs-main",
        wt.base_ref.as_deref(),
    )
    .await
    else {
        return 0;
    };
    crate::core::git::output(&wt.path, &["rev-list", "--count", &format!("{base}..HEAD")])
        .await
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i64>().ok())
        .unwrap_or(0)
}
