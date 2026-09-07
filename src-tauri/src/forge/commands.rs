use sqlx::SqlitePool;

use super::client::make_client;
use super::gitlab::fetch_and_upsert_mrs;
use crate::core::db::models::{Mr, Repo, Worktree};
use crate::core::db::store;
use crate::core::error::{AppError, AppResult, ErrorKind};

// ─── Shared lookups ───────────────────────────────────────────────────────────

/// Load the mr → worktree → repo chain for a local MR uuid. Never accept the
/// forge number: `!42` is unique per repo only.
pub(super) async fn load_mr_context(
    mr_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<(Mr, Worktree, Repo)> {
    let mr = store::mrs::get(pool, mr_id).await.map_err(|_| {
        anyhow::anyhow!(
            "no merge request with id {mr_id}. That is the id from get_mr_state or the \
             create reply — not the !42 number, which does not identify one on its own"
        )
    })?;
    let wt = store::worktrees::get(pool, &mr.worktree_id).await?;
    let repo = store::repos::get(pool, &wt.repo_id).await?;
    Ok((mr, wt, repo))
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

/// MRs for the worktree's branch: live from GitLab and upserted, else the DB rows.
#[tauri::command]
pub async fn get_mr(worktree_id: String, pool: tauri::State<'_, SqlitePool>) -> AppResult<Vec<Mr>> {
    let wt = store::worktrees::get(&*pool, &worktree_id).await?;
    let repo = store::repos::get(&*pool, &wt.repo_id).await?;

    if !repo.host.contains("github") {
        match fetch_and_upsert_mrs(&wt, &repo, &pool).await {
            Ok(mrs) => return Ok(mrs),
            Err(e) => tracing::warn!("glab mr list failed: {e}"),
        }
    }

    Ok(store::mrs::for_worktree(&*pool, &worktree_id).await?)
}

#[tauri::command]
pub async fn get_mr_threads(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<serde_json::Value> {
    mr_threads_for(&pool, &mr_id)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))
}

/// The pool-taking form, for the MCP tool.
pub async fn mr_threads_for(pool: &SqlitePool, mr_id: &str) -> anyhow::Result<serde_json::Value> {
    let (mr, _wt, repo) = load_mr_context(mr_id, pool).await?;
    make_client(&repo)
        .get_mr_threads(&repo, &mr.remote_id)
        .await
}

#[tauri::command]
pub async fn get_mr_ci(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<serde_json::Value> {
    mr_ci_for(&pool, &mr_id)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))
}

/// The pool-taking form, for the MCP tool.
pub async fn mr_ci_for(pool: &SqlitePool, mr_id: &str) -> anyhow::Result<serde_json::Value> {
    let (mr, _wt, repo) = load_mr_context(mr_id, pool).await?;
    make_client(&repo).get_mr_ci(&repo, &mr.remote_id).await
}

/// Live MR/PR fields for the overview page.
#[tauri::command]
pub async fn get_mr_details(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<serde_json::Value> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool).await?;

    let client = make_client(&repo);
    let mut details = client
        .get_mr_details(&repo, &mr.remote_id)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))?;

    // Keep the stored state in step with the live one.
    if let Some(fresh) = details["state"].as_str() {
        if fresh != mr.state {
            let _ = store::mrs::set_state(&*pool, &mr.id, fresh).await;
        }
    }

    // Fold the approval endpoint into the same payload.
    if let Ok(approval) = client.get_mr_approval(&repo, &mr.remote_id).await {
        if let Some(obj) = details.as_object_mut() {
            obj.insert("approved".into(), approval["approved"].clone());
            obj.insert("approved_by_me".into(), approval["approved_by_me"].clone());
            obj.insert("approved_by".into(), approval["approved_by"].clone());
        }
    }
    Ok(details)
}

#[tauri::command]
pub async fn reply_to_thread(
    mr_id: String,
    thread_id: String,
    body: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool).await?;

    let client = make_client(&repo);
    client
        .reply_to_thread(&repo, &mr.remote_id, &thread_id, &body)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))
}

/// Post the MR-create confirmation from the UI. Title and description stay
/// empty; the dialog collects them.
#[tauri::command]
pub async fn create_mr(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> AppResult<String> {
    let wt = store::worktrees::get(&*pool, &worktree_id).await?;

    let mut payload = crate::worktrees::op_payload(&pool, &wt).await;
    payload["title"] = serde_json::json!("");
    payload["description"] = serde_json::json!("");
    payload["target_branch"] =
        serde_json::json!(super::ops::mr_target_for(&pool, &worktree_id).await?);

    Ok(bridge
        .post(
            &pool,
            crate::approvals::ops::MR_CREATE,
            payload,
            "ui",
            Some(&wt.session_id),
        )
        .await?)
}

/// Rewrite the MR's title or description from the UI, ungated. Goes through
/// `update_mr_impl`, which re-appends the task footer.
#[tauri::command]
pub async fn edit_mr_text(
    mr_id: String,
    title: Option<String>,
    description: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    let payload = serde_json::json!({
        "mr_id": mr_id,
        "title": title,
        "description": description,
    });
    super::ops::update_mr_impl(payload, &pool)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))?;
    Ok(())
}

/// Approve the MR as the current user, ungated.
#[tauri::command]
pub async fn approve_mr(mr_id: String, pool: tauri::State<'_, SqlitePool>) -> AppResult<()> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool).await?;

    let client = make_client(&repo);
    client
        .approve_mr(&repo, &mr.remote_id)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))?;
    Ok(())
}

/// Post a comment on the MR: a general note, or a diff discussion when `file_path` and `line` are given.
#[tauri::command]
pub async fn post_mr_comment(
    mr_id: String,
    body: String,
    file_path: Option<String>,
    line: Option<i64>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool).await?;

    let client = make_client(&repo);
    let position = match (&file_path, line) {
        (Some(p), Some(l)) => Some((p.as_str(), l)),
        _ => None,
    };
    client
        .post_mr_comment(&repo, &mr.remote_id, &body, position)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))
}

// ─── MR state for Home ────────────────────────────────────────────────────────

/// Live MR state ("open"/"merged"/"closed"); `None` on a forge failure.
pub(crate) async fn mr_state(repo: &Repo, remote_id: &str) -> Option<String> {
    make_client(repo)
        .get_mr_details(repo, remote_id)
        .await
        .ok()
        .and_then(|v| v["state"].as_str().map(|s| s.to_string()))
}

#[tauri::command]
pub async fn resolve_mr_thread(
    mr_id: String,
    thread_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool).await?;

    let client = make_client(&repo);
    client
        .resolve_mr_thread(&repo, &mr.remote_id, &thread_id)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Forge))
}
