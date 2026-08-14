//! The Home snapshot: what is *locally real* right now.
//!
//! Home's job is the state Notion can't show — which repos are provisioned, what
//! the working trees look like, and where each MR stands. Everything here is
//! local git or SQLite except the MR signals, which `mr_manager` caches.
//!
//! Cost matters: this runs for every live session on each Home refresh, so it is
//! **two git calls per provisioned worktree** (`status --porcelain=v2 --branch`
//! covers dirty/staged/conflicts *and* ahead/behind; `diff --numstat` covers the
//! line delta), all issued concurrently. Tasks with no worktrees cost nothing —
//! the Notion queue is rendered from the task list the frontend already holds.

use serde::Serialize;
use sqlx::SqlitePool;

use crate::db::schema::{Mr, Repo, Task, Worktree};

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
    /// Files ticked off in a review session (drives the "3/12 viewed" progress).
    pub files_reviewed: i64,
    pub mr: Option<HomeMr>,
}

#[derive(Debug, Serialize)]
pub struct HomeEntry {
    pub short_id: String,
    pub title: String,
    pub status: String,
    /// Notion priority ("High"/"Medium"/"Low"); None for synthetic sessions.
    pub priority: Option<String>,
    /// "task" | "explorer" | "review" — same derivation as open_task.
    pub kind: String,
    pub repos: Vec<HomeRepo>,
}

/// Everything with a local footprint: any task that has worktrees, plus every
/// synthetic session (explorer / review) even before repos are attached.
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
    // The desk is synthetic too (empty page id) but it is not a session: it has no
    // worktrees and is reachable only through the console on Home.
    let tasks: Vec<Task> = sqlx::query_as(
        "SELECT * FROM tasks
         WHERE (notion_page_id = '' AND short_id != ?)
            OR short_id IN (SELECT DISTINCT task_id FROM worktrees)
         ORDER BY last_synced_at DESC",
    )
    .bind(crate::task_manager::DESK_ID)
    .fetch_all(pool)
    .await?;

    let entries = futures_util::future::join_all(
        tasks.into_iter().map(|task| async move { entry_for(task, force_mr, pool).await }),
    )
    .await;

    Ok(entries)
}

async fn entry_for(task: Task, force_mr: bool, pool: &SqlitePool) -> HomeEntry {
    let kind = if task.short_id.starts_with("review-") {
        "review"
    } else if task.notion_page_id.is_empty() {
        "explorer"
    } else {
        "task"
    };

    // Every repo attached to the task — including ones with no worktree yet, so
    // Home can show "not provisioned" rather than silently omitting them.
    let repos: Vec<Repo> = sqlx::query_as(
        "SELECT r.* FROM repos r JOIN task_repos tr ON r.id = tr.repo_id
         WHERE tr.task_id = ? ORDER BY r.project",
    )
    .bind(&task.short_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let short_id = task.short_id.clone();
    let repo_states = futures_util::future::join_all(
        repos
            .into_iter()
            .map(|repo| {
                let short_id = short_id.clone();
                async move { repo_state(&short_id, repo, force_mr, pool).await }
            }),
    )
    .await;

    HomeEntry {
        short_id: task.short_id,
        title: task.title,
        status: task.status,
        priority: task.priority,
        kind: kind.to_string(),
        repos: repo_states,
    }
}

async fn repo_state(task_id: &str, repo: Repo, force_mr: bool, pool: &SqlitePool) -> HomeRepo {
    let base = HomeRepo {
        repo_id: repo.id.clone(),
        project: repo.project.clone(),
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
        files_reviewed: 0,
        mr: None,
    };

    let wt = crate::db::load::worktree_for_repo(pool, task_id, &repo.id)
        .await
        .unwrap_or(None);

    let Some(wt) = wt else { return base };

    let mr = mr_for(&wt, &repo, force_mr, pool).await;

    if !std::path::Path::new(&wt.path).exists() {
        return HomeRepo {
            worktree_id: Some(wt.id),
            branch: Some(wt.branch),
            provisioned: true,
            missing: true,
            mr,
            ..base
        };
    }

    let (modified, staged, conflicted, ab) = working_tree_state(&wt.path).await;
    let (added, deleted, files_changed) = line_delta(&wt).await;
    let files_reviewed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reviewed_files WHERE task_id = ? AND repo_id = ?",
    )
    .bind(task_id)
    .bind(&repo.id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    // No upstream yet (never pushed) → count everything past the diff base as ahead.
    let (ahead, behind) = match ab {
        Some(ab) => ab,
        None => (commits_past_base(&wt).await, 0),
    };

    HomeRepo {
        worktree_id: Some(wt.id),
        branch: Some(wt.branch),
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
        files_reviewed,
        mr,
        ..base
    }
}

async fn mr_for(wt: &Worktree, repo: &Repo, force_mr: bool, pool: &SqlitePool) -> Option<HomeMr> {
    let mr: Mr = sqlx::query_as("SELECT * FROM mrs WHERE worktree_id = ? ORDER BY rowid DESC LIMIT 1")
        .bind(&wt.id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()?;

    let sig = crate::mr_manager::mr_signals(repo, &mr.id, &mr.remote_id, force_mr).await;
    Some(HomeMr {
        id: mr.id,
        platform: mr.platform,
        remote_id: mr.remote_id,
        state: mr.state,
        url: mr.url,
        ci: sig.ci,
        unresolved: sig.unresolved,
        approved: sig.approved,
    })
}

/// One `git status --porcelain=v2 --branch` gives dirty/staged/conflicted AND the
/// ahead/behind pair (when the branch has an upstream) — v1 porcelain would need a
/// second `rev-list`. Returns `(modified, staged, conflicted, Some((ahead, behind)))`.
async fn working_tree_state(path: &str) -> (i64, i64, i64, Option<(i64, i64)>) {
    let Ok(out) = crate::git_engine::run_git_output(path, &["status", "--porcelain=v2", "--branch"]).await
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
    let Ok(base) = crate::git_engine::diff_base(
        &wt.path,
        &wt.branch,
        "vs-main",
        wt.base_ref.as_deref(),
    )
    .await
    else {
        return (0, 0, 0);
    };

    let Ok(out) = crate::git_engine::run_git_output(
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
    let Ok(base) = crate::git_engine::diff_base(
        &wt.path,
        &wt.branch,
        "vs-main",
        wt.base_ref.as_deref(),
    )
    .await
    else {
        return 0;
    };
    crate::git_engine::run_git_output(&wt.path, &["rev-list", "--count", &format!("{base}..HEAD")])
        .await
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<i64>().ok())
        .unwrap_or(0)
}
