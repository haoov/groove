use super::types::CommitEntry;
use crate::core::db::models::Worktree;
use crate::core::db::store;
use crate::core::error::{AppError, AppResult, ErrorKind};
use sqlx::SqlitePool;
use std::collections::HashSet;

/// `git log` fields separated by the ASCII unit separator, which no name or subject holds.
const LOG_FORMAT: &str = "--format=%H%x1f%h%x1f%an%x1f%at%x1f%s";
const LOG_SEP: char = '\u{1f}';

/// Parse `git log` rows written with `LOG_FORMAT`.
fn parse_log(text: &str, task_shas: &HashSet<String>) -> Vec<CommitEntry> {
    let mut rows = vec![];
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(5, LOG_SEP).collect();
        if parts.len() == 5 {
            rows.push(CommitEntry {
                sha: parts[0].to_string(),
                short_sha: parts[1].to_string(),
                author: parts[2].to_string(),
                timestamp: parts[3].parse().unwrap_or(0),
                message: parts[4].to_string(),
                is_base: !task_shas.contains(parts[0]),
            });
        }
    }
    rows
}

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

    let per_worktree =
        futures_util::future::try_join_all(worktrees.iter().map(|wt| log_of(wt, limit))).await?;
    Ok(per_worktree.into_iter().flatten().collect())
}

/// One worktree's rows: the log and the task's own range are read together.
async fn log_of(wt: &Worktree, limit: u32) -> anyhow::Result<Vec<CommitEntry>> {
    let log_ref = wt.branch.as_str();
    let max_count = format!("--max-count={limit}");

    let log_args = ["log", &max_count, LOG_FORMAT, log_ref];
    let (output, task_shas) = tokio::join!(
        crate::core::git::output(&wt.path, &log_args),
        task_shas(wt, log_ref),
    );

    let output = output?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git log {log_ref} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(parse_log(
        &String::from_utf8_lossy(&output.stdout),
        &task_shas,
    ))
}

/// The task's own commits: `base..ref`. A pinned `base_ref` is the base; `None`
/// means the remote has no base branch.
async fn task_shas(wt: &Worktree, log_ref: &str) -> HashSet<String> {
    let base = crate::core::git::refs::upstream_base(&wt.path, wt.base_ref.as_deref())
        .await
        .ok();
    let range = match &base {
        Some(b) => format!("{b}..{log_ref}"),
        None => log_ref.to_string(),
    };
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
        .unwrap_or_default()
}

#[tauri::command]
pub async fn get_commit_log(
    task_id: String,
    worktree_id: Option<String>,
    limit: Option<u32>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Vec<CommitEntry>> {
    get_commit_log_impl(&task_id, worktree_id.as_deref(), limit.unwrap_or(50), &pool)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))
}

pub async fn get_commit_log_mcp(
    task_id: &str,
    limit: u32,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<CommitEntry>> {
    get_commit_log_impl(task_id, None, limit, pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_author_name_with_a_pipe_keeps_its_row_intact() {
        let text = "abc123\u{1f}abc123\u{1f}Ada | Lovelace\u{1f}1700000000\u{1f}feat: a | b\n";
        let rows = parse_log(text, &HashSet::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].author, "Ada | Lovelace");
        assert_eq!(rows[0].message, "feat: a | b");
        assert_eq!(rows[0].timestamp, 1700000000);
        assert!(rows[0].is_base, "a sha outside the range is base");
    }

    #[test]
    fn the_log_format_matches_the_separator_the_parser_splits_on() {
        assert_eq!(LOG_FORMAT.matches("%x1f").count(), 4);
        assert!(!LOG_FORMAT.contains('|'));
        assert_eq!(LOG_SEP, '\u{1f}');
    }

    #[test]
    fn a_short_row_is_dropped() {
        assert!(parse_log("abc\u{1f}abc\u{1f}Ada\n", &HashSet::new()).is_empty());
    }
}
