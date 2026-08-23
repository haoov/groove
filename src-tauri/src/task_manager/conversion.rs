//! Explorer → task conversion.
//!
//! An explorer is a throwaway local session; converting it creates the real
//! Notion page and moves the whole local footprint (worktrees on disk, DB rows,
//! the Claude conversation) onto the new task id. Every step is independently
//! recoverable: a worktree that fails to switch or move keeps working where it
//! is and reports a warning instead of aborting the conversion.

use sqlx::SqlitePool;

use crate::core::db::models::{NotionTask, Repo, SessionKind, Worktree};
use crate::core::db::store;

/// The confirmation payload: which explorer to convert, plus the task to file for
/// it. The task half is shared with `task.create` — see creation.rs.
struct ConvertRequest<'a> {
    explorer_id: &'a str,
    task: crate::provider::notion::NewTask<'a>,
}

impl<'a> ConvertRequest<'a> {
    fn from_payload(payload: &'a serde_json::Value) -> anyhow::Result<Self> {
        Ok(Self {
            explorer_id: payload["explorer_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing explorer_id"))?,
            task: crate::provider::notion::NewTask::from_payload(payload)?,
        })
    }
}

/// Refuse anything that isn't a live explorer session.
///
/// This op re-points a session's worktrees/repos/annotations onto a new task
/// and re-keys the source row — catastrophic if aimed at a real task or a
/// review session. The id comes from the backend's active session, so validate
/// it rather than trusting the caller.
async fn validate_source(explorer_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    match store::sessions::kind_of(pool, explorer_id).await? {
        None => Err(anyhow::anyhow!("no session {explorer_id} to convert")),
        Some(SessionKind::Explorer) => Ok(()),
        Some(kind) => Err(anyhow::anyhow!(
            "{explorer_id} is a {kind:?} session — only explorer sessions convert to tasks"
        )),
    }
}

/// Carry the explorer branch — its commits and uncommitted edits — over to the
/// task's branch name. A rename, not a new branch: the history IS the work.
async fn rename_branch(wt: &Worktree, new_branch: &str) -> Result<(), String> {
    // A stray local branch named "HEAD" (older refresh bug) makes branch ops
    // fail with "refname 'HEAD' is ambiguous" — clean it before renaming.
    crate::worktrees::repair_head_branch(&wt.path).await;

    match crate::core::git::output(&wt.path, &["branch", "-m", new_branch]).await {
        Ok(o) if o.status.success() => {
            crate::core::git::cache::flush();
            Ok(())
        }
        Ok(o) => Err(format!(
            "could not rename the branch of {} to {new_branch}: {}",
            wt.path,
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("git branch -m failed for {}: {e}", wt.path)),
    }
}

/// Relocate `<root>/<explorer_id>/<project>` → `<root>/<short_id>/<project>`,
/// driven from the MAIN clone. Returns the worktree's new path.
async fn relocate_worktree(
    wt: &Worktree,
    repo: &Repo,
    session_dir: &std::path::Path,
    new_branch: &str,
) -> Result<String, String> {
    let dest = session_dir
        .join(crate::worktrees::naming::worktree_dir(&repo.project, new_branch))
        .to_string_lossy()
        .to_string();
    let _ = std::fs::create_dir_all(session_dir);
    match crate::core::git::output(
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
    session_dir: &std::path::Path,
    new_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<(Vec<(String, String)>, Vec<String>)> {
    let worktrees = store::worktrees::for_session(pool, explorer_id).await?;

    let mut switched: Vec<(String, String)> = vec![];
    let mut warnings: Vec<String> = vec![];

    for wt in &worktrees {
        if let Err(msg) = rename_branch(wt, new_branch).await {
            tracing::error!("[convert] {msg}");
            warnings.push(msg);
            continue;
        }

        // Failure to move keeps the old path (still functional).
        let mut final_path = wt.path.clone();
        if let Some(repo) = store::repos::get_opt(pool, &wt.repo_id).await? {
            match relocate_worktree(wt, &repo, session_dir, new_branch).await {
                Ok(dest) => final_path = dest,
                Err(msg) => {
                    tracing::warn!("[convert] {msg}");
                    warnings.push(msg);
                }
            }
        }
        switched.push((wt.id.clone(), final_path));
    }

    Ok((switched, warnings))
}

/// Hand the explorer's Claude session to the new task id via the legacy
/// resume-fallback file, so reopening the task resumes the same conversation.
fn handoff_agent_session(explorer_id: &str, session_dir: &std::path::Path) {
    let session_uuid = crate::agent_manager::task_session_uuid(explorer_id);
    let sid_path = session_dir.join(".agent_session_id");
    let _ = std::fs::create_dir_all(session_dir);
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
/// The session shape the branch namer needs, before the row is re-keyed.
fn adopted_session(short_id: &str, task: &crate::provider::notion::NewTask<'_>) -> crate::core::db::models::Session {
    crate::core::db::models::Session {
        id: short_id.to_string(),
        kind: crate::core::db::models::SessionKind::Task,
        title: task.title.to_string(),
        notion_page_id: Some(String::new()),
        review_project: None,
        review_iid: None,
        created_at: 0,
    }
}

pub async fn create_task_from_explorer_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let req = ConvertRequest::from_payload(&payload)?;
    validate_source(req.explorer_id, pool).await?;

    // Same page creation as filing a standalone task (see notion::create) — this
    // op is that, plus adopting the session onto the result.
    let cfg = crate::core::config::require()?;
    let (notion_page_id, short_id) =
        crate::provider::notion::create::create_page(&cfg.notion.token, &req.task).await?;

    let now = chrono::Utc::now().timestamp();
    let new_branch = crate::worktrees::naming::default_branch(&adopted_session(&short_id, &req.task));
    let session_dir = crate::worktrees::session_dir(&short_id);

    let (switched, branch_warnings) =
        promote_worktrees(req.explorer_id, &session_dir, &new_branch, pool).await?;

    let task = NotionTask {
        page_id: notion_page_id.clone(),
        short_id: short_id.clone(),
        title: req.task.title.to_string(),
        status: req.task.status_value.to_string(),
        priority: None,
        synced_at: now,
    };
    store::sessions::adopt_explorer(pool, req.explorer_id, &task, &switched, &new_branch).await?;

    handoff_agent_session(req.explorer_id, &session_dir);
    cleanup_explorer_dir(&crate::worktrees::session_dir(req.explorer_id));

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
