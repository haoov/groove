use sqlx::SqlitePool;
use crate::core::db::models::Worktree;
use crate::core::db::store;
use super::types::CommitEntry;

pub(super) async fn get_commit_log_impl(
    task_id: &str,
    worktree_id: Option<&str>,
    limit: u32,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<CommitEntry>> {
    // Scope to a single worktree when given, otherwise every worktree.
    let worktrees: Vec<Worktree> = match worktree_id {
        Some(wid) => {
            let wt = store::worktrees::get(pool, wid).await?;
            if wt.session_id == task_id { vec![wt] } else { vec![] }
        }
        None => store::worktrees::for_session(pool, task_id).await?,
    };

    let mut all = vec![];
    for wt in worktrees {
        let log_ref = wt.branch.as_str();

        // Review worktrees pin the MR's target branch as the base, so history
        // divides at the real target and not the repo default. `None` means the
        // remote has no branch to divide at (origin-less or unfetched clone), and
        // then every commit here is local work rather than base history.
        let base = crate::core::git::refs::upstream_base(&wt.path, wt.base_ref.as_deref()).await.ok();

        // Full recent history of the branch — task commits AND upstream base
        // commits — so the list shows where the branch grew from.
        let max_count = format!("--max-count={limit}");
        // Subject (%s) goes LAST: it's the only field that can contain `|`, and
        // splitn keeps the remainder intact only for the final field.
        let output = crate::core::git::output(
            &wt.path,
            &["log", &max_count, "--format=%H|%h|%an|%at|%s", log_ref],
        )
        .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "git log {log_ref} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        // Which of those are the task's own work: reachable from the branch but
        // not from the upstream base (base..ref). Everything else is base history.
        let range = match &base {
            Some(b) => format!("{b}..{log_ref}"),
            None => log_ref.to_string(),
        };
        let task_shas: std::collections::HashSet<String> =
            crate::core::git::output(&wt.path, &["rev-list", &range])
                .await
                .ok()
                .filter(|o| o.status.success())
                .map(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .map(|l| l.trim().to_string())
                        .collect()
                })
                .unwrap_or_default();

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let parts: Vec<&str> = line.splitn(5, '|').collect();
            if parts.len() == 5 {
                all.push(CommitEntry {
                    sha: parts[0].to_string(),
                    short_sha: parts[1].to_string(),
                    author: parts[2].to_string(),
                    timestamp: parts[3].parse().unwrap_or(0),
                    message: parts[4].to_string(),
                    is_base: !task_shas.contains(parts[0]),
                });
            }
        }
    }
    Ok(all)
}

#[tauri::command]
pub async fn get_commit_log(
    task_id: String,
    worktree_id: Option<String>,
    limit: Option<u32>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<CommitEntry>, String> {
    get_commit_log_impl(&task_id, worktree_id.as_deref(), limit.unwrap_or(50), &pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_commit_log_mcp(
    task_id: &str,
    limit: u32,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<CommitEntry>> {
    get_commit_log_impl(task_id, None, limit, pool).await
}
