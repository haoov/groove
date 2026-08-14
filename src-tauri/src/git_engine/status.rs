use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorktreeStatus {
    pub worktree_id: String,
    pub modified: usize,
    pub staged: usize,
    pub ahead: i64,
    pub behind: i64,
    pub remote_branch_gone: bool,
}

/// Cache of the `remote_branch_gone` ls-remote (network) result per (path, branch),
/// so repeated status refreshes during agent edit bursts don't hammer the network.
type RemoteGoneCache = HashMap<(String, String), (Instant, bool)>;
static REMOTE_GONE_CACHE: LazyLock<Mutex<RemoteGoneCache>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const REMOTE_GONE_TTL: Duration = Duration::from_secs(60);

fn cached_remote_gone(path: &str, branch: &str) -> Option<bool> {
    let guard = REMOTE_GONE_CACHE.lock().ok()?;
    let (ts, val) = guard.get(&(path.to_string(), branch.to_string()))?;
    if ts.elapsed() < REMOTE_GONE_TTL {
        Some(*val)
    } else {
        None
    }
}

fn store_remote_gone(path: &str, branch: &str, val: bool) {
    if let Ok(mut guard) = REMOTE_GONE_CACHE.lock() {
        guard.insert((path.to_string(), branch.to_string()), (Instant::now(), val));
    }
}

#[tauri::command]
pub async fn get_worktree_status(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<WorktreeStatus, String> {
    let wt = crate::db::load::worktree(&pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let status_out = super::run_git_output(&wt.path, &["status", "--porcelain"])
        .await
        .map_err(|e| e.to_string())?;

    let mut modified = 0usize;
    let mut staged = 0usize;
    for line in String::from_utf8_lossy(&status_out.stdout).lines() {
        if line.len() >= 2 {
            let x = line.chars().next().unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');
            if x == '?' && y == '?' {
                // Untracked (newly created) file — count it as a working-tree change.
                modified += 1;
            } else {
                if x != ' ' { staged += 1; }
                if y != ' ' { modified += 1; }
            }
        }
    }

    // Ahead/behind vs the branch's own remote tracking ref (origin/<branch>),
    // not vs main — that distance powers the diff view instead.
    let upstream = format!("origin/{}", wt.branch);
    let has_upstream = super::run_git_output(&wt.path, &["rev-parse", "--verify", &upstream])
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    let (ahead, behind) = if has_upstream {
        let range = format!("HEAD...{upstream}");
        super::run_git_output(&wt.path, &["rev-list", "--left-right", "--count", &range])
            .await
            .ok()
            .and_then(|o| {
                if !o.status.success() { return None; }
                let s = String::from_utf8(o.stdout).ok()?;
                let parts: Vec<&str> = s.split_whitespace().collect();
                if parts.len() == 2 {
                    Some((parts[0].parse::<i64>().ok()?, parts[1].parse::<i64>().ok()?))
                } else {
                    None
                }
            })
            .unwrap_or((0, 0))
    } else {
        // Never pushed: every commit beyond the base is unpushed. Honours a review
        // worktree's pinned target, so this count agrees with the diff instead of
        // measuring against the repo default. No base on origin means there is
        // nothing to measure against, and the count stays 0.
        let ahead = match super::upstream_base(&wt.path, wt.base_ref.as_deref()).await {
            Ok(base_ref) => super::run_git_output(&wt.path, &["rev-list", "--count", &format!("{base_ref}..HEAD")])
                .await
                .ok()
                .and_then(|o| {
                    if !o.status.success() { return None; }
                    String::from_utf8(o.stdout).ok()?.trim().parse::<i64>().ok()
                })
                .unwrap_or(0),
            Err(_) => 0,
        };
        (ahead, 0)
    };

    let remote_branch_gone = if let Some(cached) = cached_remote_gone(&wt.path, &wt.branch) {
        cached
    } else {
        let branch = wt.branch.clone();
        let path_for_remote = wt.path.clone();
        let val = tokio::task::spawn_blocking(move || {
            // Only flag gone if git has ever seen this branch on remote
            let remote_ref = format!("refs/remotes/origin/{branch}");
            let was_pushed = std::process::Command::new("git")
                .args(["rev-parse", "--verify", &remote_ref])
                .current_dir(&path_for_remote)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !was_pushed {
                return false;
            }

            std::process::Command::new("git")
                .args(["ls-remote", "--heads", "origin", &branch])
                .current_dir(&path_for_remote)
                .output()
                .map(|out| out.status.success() && out.stdout.is_empty())
                .unwrap_or(false)
        })
        .await
        .unwrap_or(false);
        store_remote_gone(&wt.path, &wt.branch, val);
        val
    };

    Ok(WorktreeStatus { worktree_id, modified, staged, ahead, behind, remote_branch_gone })
}
