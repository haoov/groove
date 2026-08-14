mod base_ref;
mod blame;
mod commits;
mod diff;
mod ops;
mod parse;
mod status;
mod types;
mod watcher;
mod worktrees;

// Glob re-exports are required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function, so they must all be available at git_engine::*.
// Base-ref resolution is internal: nothing outside the crate asks for it.
pub(crate) use base_ref::*;
pub use blame::*;
pub use commits::*;
pub use diff::*;
pub use ops::*;
pub use status::*;
// The DTOs the commands return; other modules construct BranchSpec / read MainRepo.
pub use types::*;
pub use watcher::*;
pub use worktrees::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sqlx::SqlitePool;
use crate::db::schema::Worktree;

/// Run `git <args>` in `dir` off the async runtime and return its raw Output,
/// for callers that inspect status/stderr themselves. All git invocations in
/// this module go through here so they never block a tokio worker thread.
pub(crate) async fn run_git_output(dir: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let dir = dir.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let joined = args.join(" ");
    tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(&args)
            // Force English, machine-stable output: several call sites match on
            // git's messages (e.g. "already exists" when a worktree is being
            // re-provisioned), and on a localized system git answers in the
            // user's language ("existe déjà"), silently breaking those checks.
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .current_dir(&dir)
            .output()
    })
    .await?
    .map_err(|e| anyhow::anyhow!("failed to run git {joined}: {e}"))
}

/// Run `git <args>` in `dir` off the async runtime, check the exit status, and
/// return stdout. A non-zero exit becomes an error carrying stderr.
pub(crate) async fn run_git(dir: &str, args: &[&str]) -> anyhow::Result<String> {
    let joined = args.join(" ");
    let output = run_git_output(dir, args).await?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git {joined} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Insert or update the worktree row for a (task, repo) pair and return it.
/// Shared by the task and explorer provisioners. Fetches by (task_id, repo_id)
/// so it returns the correct row even when the upsert took the ON CONFLICT path.
pub(crate) async fn upsert_worktree(
    task_id: &str,
    repo_id: &str,
    branch: &str,
    path: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Worktree> {
    let wt_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO worktrees (id, task_id, repo_id, branch, path, is_active, created_at)
         VALUES (?, ?, ?, ?, ?, 1, ?)
         ON CONFLICT(task_id, repo_id) DO UPDATE SET
           branch = excluded.branch,
           path   = excluded.path,
           is_active = 1",
    )
    .bind(&wt_id)
    .bind(task_id)
    .bind(repo_id)
    .bind(branch)
    .bind(path)
    .bind(now)
    .execute(pool)
    .await?;

    crate::db::load::worktree_for_repo(pool, task_id, repo_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("no worktree for {task_id} in {repo_id}"))
}

pub struct State {
    pub(crate) inner: Arc<StateInner>,
}

pub(crate) struct StateInner {
    // worktree path → notify watcher, kept alive for the lifetime of the session.
    // Keyed by path so re-opening a session doesn't spawn duplicate watchers.
    pub(crate) watchers: Mutex<HashMap<String, notify::RecommendedWatcher>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                watchers: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
