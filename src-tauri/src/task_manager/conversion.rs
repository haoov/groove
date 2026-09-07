//! Explorer → task conversion: file the task at its provider, then move the
//! worktrees, DB rows and agent session onto the new task id.

use sqlx::SqlitePool;

use crate::core::db::models::{Repo, SessionKind, Worktree};
use crate::core::db::store;
use crate::provider::types::{ProviderId, TaskDraft};

/// The confirmation payload: the explorer to convert and the task to file.
struct ConvertRequest<'a> {
    explorer_id: &'a str,
    provider: ProviderId,
    draft: TaskDraft<'a>,
}

impl<'a> ConvertRequest<'a> {
    fn from_payload(payload: &'a serde_json::Value) -> anyhow::Result<Self> {
        let provider = crate::provider::commands::draft_provider(payload)?;
        Ok(Self {
            explorer_id: payload["explorer_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing explorer_id"))?,
            provider,
            draft: TaskDraft {
                title: payload["title"].as_str().unwrap_or("Untitled task"),
                body_markdown: payload["body_markdown"].as_str().unwrap_or(""),
                repo: payload["repo"].as_str(),
            },
        })
    }
}

/// Refuse anything that is not a live explorer session.
async fn validate_source(explorer_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    match store::sessions::kind_of(pool, explorer_id).await? {
        None => Err(anyhow::anyhow!("no session {explorer_id} to convert")),
        Some(SessionKind::Explorer) => Ok(()),
        Some(kind) => Err(anyhow::anyhow!(
            "{explorer_id} is a {kind:?} session — only explorer sessions convert to tasks"
        )),
    }
}

/// Rename the explorer branch to the task's branch name.
async fn rename_branch(wt: &Worktree, new_branch: &str) -> Result<(), String> {
    // A stray local branch named "HEAD" makes `branch -m` fail as ambiguous.
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

/// Move the worktree into the task's session dir; returns the new path.
async fn relocate_worktree(
    wt: &Worktree,
    repo: &Repo,
    session_dir: &std::path::Path,
    new_branch: &str,
) -> Result<String, String> {
    let dest_path = session_dir.join(crate::worktrees::naming::worktree_dir(
        &repo.project,
        new_branch,
    ));
    let dest = dest_path.to_string_lossy().to_string();
    // `git worktree move` does not create missing parent directories.
    if let Some(parent) = dest_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match crate::core::git::output(&repo.local_path, &["worktree", "move", &wt.path, &dest]).await {
        Ok(o) if o.status.success() => Ok(dest),
        Ok(o) => Err(format!(
            "could not move {} to {dest}: {}",
            wt.path,
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("worktree move failed for {}: {e}", wt.path)),
    }
}

/// Rename and relocate every worktree of the explorer. Returns the
/// `(worktree id, final path)` pairs that switched, and a warning per failure.
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

/// Write the explorer's agent session id into the task's `.agent_session_id` file.
fn handoff_agent_session(explorer_id: &str, session_dir: &std::path::Path) {
    let session_uuid = crate::agent_manager::task_session_uuid(explorer_id);
    let sid_path = session_dir.join(".agent_session_id");
    let _ = std::fs::create_dir_all(session_dir);
    if let Err(e) = std::fs::write(&sid_path, session_uuid) {
        tracing::warn!(
            "[convert] could not persist agent session handoff at {}: {e}",
            sid_path.display()
        );
    }
}

/// Remove the empty explorer dir. Keep `remove_dir`: a worktree that failed to move stays inside.
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

/// The session shape the branch namer needs, before the row is re-keyed.
fn adopted_session(short_id: &str, title: &str) -> crate::core::db::models::Session {
    crate::core::db::models::Session {
        id: short_id.to_string(),
        kind: crate::core::db::models::SessionKind::Task,
        title: title.to_string(),
        external_id: Some(String::new()),
        review_project: None,
        review_iid: None,
        created_at: 0,
    }
}

/// Convert an explorer session into a task; bridge op `task.create_from_explorer`.
pub async fn create_task_from_explorer_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let req = ConvertRequest::from_payload(&payload)?;
    validate_source(req.explorer_id, pool).await?;

    let provider = crate::provider::get(req.provider)?;
    let filed = provider.create_task(&req.draft).await?;
    let short_id =
        crate::provider::commands::mint_short_id(pool, provider, &filed, &mut Default::default())
            .await?;

    let now = chrono::Utc::now().timestamp();
    let new_branch = crate::worktrees::naming::default_branch(
        &adopted_session(&short_id, req.draft.title),
        filed.branch_tag.as_deref(),
    );
    let session_dir = crate::worktrees::session_dir(&short_id);

    let (switched, branch_warnings) =
        promote_worktrees(req.explorer_id, &session_dir, &new_branch, pool).await?;

    let task = crate::provider::mirror_row(&short_id, &filed);
    store::sessions::adopt_explorer(pool, req.explorer_id, &task, &switched, &new_branch).await?;

    handoff_agent_session(req.explorer_id, &session_dir);
    cleanup_explorer_dir(&crate::worktrees::session_dir(req.explorer_id));

    let mut out = crate::provider::commands::filed_response(&short_id, &filed, now);
    out["branch_warnings"] = serde_json::json!(branch_warnings);
    Ok(out)
}
