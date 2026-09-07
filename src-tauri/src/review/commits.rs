use super::types::CommitEntry;
use crate::core::db::models::Worktree;
use crate::core::db::store;
use sqlx::SqlitePool;

pub(super) async fn get_commit_log_impl(
    task_id: &str,
    worktree_id: Option<&str>,
    limit: u32,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<CommitEntry>> {
    let worktrees: Vec<Worktree> = match worktree_id {
        Some(wid) => {
            let wt = store::worktrees::get(pool, wid).await?;
            if wt.session_id == task_id {
                vec![wt]
            } else {
                vec![]
            }
        }
        None => store::worktrees::for_session(pool, task_id).await?,
    };

    let mut all = vec![];
    for wt in worktrees {
        let log_ref = wt.branch.as_str();

        // A pinned `base_ref` is the base; `None` means the remote has no base branch.
        let base = crate::core::git::refs::upstream_base(&wt.path, wt.base_ref.as_deref())
            .await
            .ok();

        let max_count = format!("--max-count={limit}");
        // Subject (%s) goes last: it is the only field that can contain `|`.
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

        // The task's own commits: `base..ref`.
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
