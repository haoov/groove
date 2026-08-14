//! Explorer → task conversion.
//!
//! An explorer is a throwaway local session; converting it creates the real
//! Notion page and moves the whole local footprint (worktrees on disk, DB rows,
//! the Claude conversation) onto the new task id. Every step is independently
//! recoverable: a worktree that fails to switch or move keeps working where it
//! is and reports a warning instead of aborting the conversion.

use sqlx::SqlitePool;

use crate::db::schema::{Repo, Worktree};

/// DB tables keyed by `task_id` that must follow the session to its new id.
/// Kept in sync with `delete_task_rows` in commands.rs.
const OWNED_TABLES: [&str; 7] = [
    "worktrees",
    "task_repos",
    "annotations",
    "tab_snapshots",
    "agent_sessions",
    "reviewed_files",
    "task_time",
];

/// The confirmation payload: which explorer to convert, plus the task to file for
/// it. The task half is shared with `task.create` — see creation.rs.
struct ConvertRequest<'a> {
    explorer_id: &'a str,
    task: super::creation::NewTask<'a>,
}

impl<'a> ConvertRequest<'a> {
    fn from_payload(payload: &'a serde_json::Value) -> anyhow::Result<Self> {
        Ok(Self {
            explorer_id: payload["explorer_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing explorer_id"))?,
            task: super::creation::NewTask::from_payload(payload)?,
        })
    }
}

/// Refuse anything that isn't a live explorer session.
///
/// This op re-points a session's worktrees/repos/annotations onto a new task and
/// deletes the source row — catastrophic if aimed at a real task or a review
/// session. The id comes from the backend's active task, so validate it rather
/// than trusting the caller.
async fn validate_source(explorer_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    let source = crate::db::load::task_opt(pool, explorer_id).await?;
    match source {
        None => Err(anyhow::anyhow!("no session {explorer_id} to convert")),
        Some(t) if !t.notion_page_id.is_empty() => Err(anyhow::anyhow!(
            "{explorer_id} is a real task (already in Notion), not an explorer session — \
             focus the explorer you want to convert and retry"
        )),
        Some(_) if !explorer_id.starts_with("explorer-") => Err(anyhow::anyhow!(
            "{explorer_id} is not an explorer session (review sessions can't be converted)"
        )),
        Some(_) => Ok(()),
    }
}

/// Move a detached explorer worktree onto a real branch, carrying over
/// uncommitted edits and any detached-HEAD commits.
async fn switch_to_branch(wt: &Worktree, new_branch: &str) -> Result<(), String> {
    // A stray local branch named "HEAD" (older refresh bug) makes `git switch`
    // fail with "refname 'HEAD' is ambiguous" — clean it before switching.
    crate::git_engine::repair_head_branch(&wt.path).await;

    match crate::git_engine::run_git_output(&wt.path, &["switch", "-c", new_branch]).await {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(format!(
            "could not switch {} to {new_branch}: {}",
            wt.path,
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("git switch failed for {}: {e}", wt.path)),
    }
}

/// Relocate `<root>/<explorer_id>/<project>` → `<root>/<short_id>/<project>`,
/// driven from the MAIN clone. Returns the worktree's new path.
async fn relocate_worktree(
    wt: &Worktree,
    repo: &Repo,
    task_dir: &std::path::Path,
) -> Result<String, String> {
    let dest = task_dir.join(&repo.project).to_string_lossy().to_string();
    let _ = std::fs::create_dir_all(task_dir);
    match crate::git_engine::run_git_output(
        &repo.local_path,
        &["worktree", "move", &wt.path, &dest],
    )
    .await
    {
        Ok(o) if o.status.success() => Ok(dest),
        Ok(o) => Err(format!(
            "could not move {} to {dest}: {}",
            wt.path,
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("worktree move failed for {}: {e}", wt.path)),
    }
}

/// Branch + relocate every worktree of the explorer.
///
/// Returns the `(worktree id, final path)` pairs that reached `new_branch` and
/// the warnings for the ones that didn't — those keep their old branch and path
/// and stay usable, so a partial failure is not fatal to the conversion.
async fn promote_worktrees(
    explorer_id: &str,
    task_dir: &std::path::Path,
    new_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<(Vec<(String, String)>, Vec<String>)> {
    let worktrees = crate::db::load::all_worktrees(pool, explorer_id).await?;

    let mut branched: Vec<(String, String)> = vec![];
    let mut warnings: Vec<String> = vec![];

    for wt in &worktrees {
        if let Err(msg) = switch_to_branch(wt, new_branch).await {
            tracing::error!("[convert] {msg}");
            warnings.push(msg);
            continue;
        }

        // Failure to move keeps the old path (still functional).
        let mut final_path = wt.path.clone();
        let repo = crate::db::load::repo_opt(pool, &wt.repo_id).await?;
        if let Some(repo) = repo {
            match relocate_worktree(wt, &repo, task_dir).await {
                Ok(dest) => final_path = dest,
                Err(msg) => {
                    tracing::warn!("[convert] {msg}");
                    warnings.push(msg);
                }
            }
        }
        branched.push((wt.id.clone(), final_path));
    }

    Ok((branched, warnings))
}

/// Insert the real task and move every owned row onto it, then drop the explorer
/// row — one transaction, so the session is never split across two ids.
async fn repoint_rows(
    req: &ConvertRequest<'_>,
    short_id: &str,
    notion_page_id: &str,
    now: i64,
    new_branch: &str,
    branched: &[(String, String)],
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO tasks (short_id, notion_page_id, title, status, priority, last_synced_at)
         VALUES (?, ?, ?, ?, NULL, ?)
         ON CONFLICT(short_id) DO UPDATE SET
           notion_page_id = excluded.notion_page_id,
           title          = excluded.title,
           status         = excluded.status,
           last_synced_at = excluded.last_synced_at",
    )
    .bind(short_id)
    .bind(notion_page_id)
    .bind(req.task.title)
    .bind(req.task.status_value)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    for table in OWNED_TABLES {
        sqlx::query(&format!("UPDATE {table} SET task_id = ? WHERE task_id = ?"))
            .bind(short_id)
            .bind(req.explorer_id)
            .execute(&mut *tx)
            .await?;
    }

    // Persist the new branch + relocated path on the worktrees we switched.
    for (wt_id, path) in branched {
        sqlx::query("UPDATE worktrees SET branch = ?, path = ? WHERE id = ?")
            .bind(new_branch)
            .bind(path)
            .bind(wt_id)
            .execute(&mut *tx)
            .await?;
    }

    // Scoped like discard_explorer — never delete a real task's row.
    sqlx::query("DELETE FROM tasks WHERE short_id = ? AND notion_page_id = ''")
        .bind(req.explorer_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Hand the explorer's Claude session to the new task id via the legacy
/// resume-fallback file, so reopening the task resumes the same conversation.
fn handoff_agent_session(explorer_id: &str, task_dir: &std::path::Path) {
    let session_uuid = crate::agent_manager::task_session_uuid(explorer_id);
    let sid_path = task_dir.join(".agent_session_id");
    let _ = std::fs::create_dir_all(task_dir);
    if let Err(e) = std::fs::write(&sid_path, session_uuid) {
        // Not fatal, but the explorer's conversation won't resume under the task.
        tracing::warn!(
            "[convert] could not persist agent session handoff at {}: {e}",
            sid_path.display()
        );
    }
}

/// Remove the now-defunct explorer dir. `remove_dir` (not `_all`) so a worktree
/// that failed to move keeps its directory.
fn cleanup_explorer_dir(explorer_dir: &std::path::Path) {
    let _ = std::fs::remove_file(explorer_dir.join(".agent_session_id"));
    if let Err(e) = std::fs::remove_dir(explorer_dir) {
        if explorer_dir.exists() {
            tracing::warn!(
                "[convert] could not remove explorer dir {}: {e}",
                explorer_dir.display()
            );
        }
    }
}

/// Convert an explorer session into a real task. Called from the confirmation
/// bridge (op `task.create_from_explorer`); returns the new task as JSON
/// (delivered to both the agent and the frontend).
pub async fn create_task_from_explorer_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let req = ConvertRequest::from_payload(&payload)?;
    validate_source(req.explorer_id, pool).await?;

    // Same page creation as filing a standalone task (see creation.rs) — this op
    // is that, plus adopting the session onto the result.
    let (notion_page_id, short_id) = super::creation::create_page(&req.task).await?;

    let now = chrono::Utc::now().timestamp();
    let new_branch = super::derive_branch(&short_id);
    let worktree_root = crate::git_engine::resolve_worktree_root();
    let task_dir = worktree_root.join(&short_id);

    let (branched, branch_warnings) =
        promote_worktrees(req.explorer_id, &task_dir, &new_branch, pool).await?;

    repoint_rows(&req, &short_id, &notion_page_id, now, &new_branch, &branched, pool).await?;

    handoff_agent_session(req.explorer_id, &task_dir);
    cleanup_explorer_dir(&worktree_root.join(req.explorer_id));

    Ok(serde_json::json!({
        "short_id": short_id,
        "notion_page_id": notion_page_id,
        "title": req.task.title,
        "status": req.task.status_value,
        "priority": null,
        "last_synced_at": now,
        "branch_warnings": branch_warnings,
    }))
}
