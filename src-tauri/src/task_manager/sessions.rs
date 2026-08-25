//! Synthetic, task-less sessions: explorers and MR reviews.
//!
//! Both are `sessions` rows with no task behind them, which is what lets
//! them reuse the entire task machinery (worktrees, agent, diff, add-repo)
//! without ever appearing in a task source. `sessions.kind` is the discriminator.

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::db::models::{Repo, SessionKind};
use crate::core::db::store;
use super::commands::open_task_impl;
use super::State;

fn new_explorer_id() -> String {
    let uid = uuid::Uuid::new_v4().simple().to_string();
    format!("explorer-{}", &uid[..8])
}

#[tauri::command]
pub async fn open_explorer_session(
    app: tauri::AppHandle,
    name: Option<String>,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    let short_id = new_explorer_id();
    let title = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Explorer {}", &short_id["explorer-".len()..]));

    let session = store::sessions::create_explorer(&*pool, &short_id, &title)
        .await
        .map_err(|e| e.to_string())?;
    let task = store::sessions::view(&*pool, &session.id)
        .await
        .map_err(|e| e.to_string())?;

    task_state.set_active_task_id(Some(short_id.clone()));

    app.emit(
        crate::core::events::WORKSPACE_READY,
        serde_json::json!({
            "task": task,
            "worktrees": [],
            "repos": [],
            "kind": SessionKind::Explorer,
        }),
    )
    .map_err(|e| e.to_string())?;

    Ok(short_id)
}

// ─── Review sessions ──────────────────────────────────────────────────────────
// A review session checks out an MR's source branch, with the MR's target
// pinned as the diff/log base. `(project, iid)` is the identity: reopening the
// same MR resumes the session, annotations intact. Finishing discards it.

fn review_session_id(project_full: &str, iid: u64) -> String {
    format!("review-{}-{iid}", project_full.replace('/', "-"))
}

/// Register the MR's MAIN clone and attach it to the session. The slug is the
/// clone's place in the pool: the review queue matched it there by
/// `<host>/<project_full>`.
async fn attach_review_repo(
    session_id: &str,
    host: &str,
    project_full: &str,
    local_path: String,
    pool: &SqlitePool,
) -> anyhow::Result<Repo> {
    let slug = format!("{host}/{project_full}");
    let repo = crate::worktrees::register_repo_impl(&slug, local_path, pool).await?;
    store::repos::attach(pool, session_id, &repo.id).await?;
    Ok(repo)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn open_review_session(
    app: tauri::AppHandle,
    project_full: String,
    iid: u64,
    title: String,
    source_branch: String,
    target_branch: String,
    web_url: String,
    local_path: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    // Both first open and resume run the same provisioning: every step is
    // idempotent, and re-running it is what makes closing and reopening a
    // review reliable — the worktree may have been pruned and the MR has
    // probably moved on.
    let open = async {
        let session = store::sessions::upsert_review(
            &*pool,
            &review_session_id(&project_full, iid),
            &project_full,
            iid as i64,
            &title,
        )
        .await?;
        let host = crate::core::git::url_host(&web_url)
            .ok_or_else(|| anyhow::anyhow!("no host in MR url {web_url}"))?;
        let repo = attach_review_repo(&session.id, &host, &project_full, local_path, &pool).await?;
        let wt = crate::worktrees::provision_review_worktree(
            &session.id,
            &repo,
            &source_branch,
            &target_branch,
            &pool,
        )
        .await?;

        // Bind the MR so the Forge / MR-overview machinery works from the first
        // render. The forge comes from the URL, not a constant: a GitHub review
        // stored as 'gitlab' is what the UI reads to pick `#42` over `!42`.
        let platform = if web_url.contains("github") { "github" } else { "gitlab" };
        store::mrs::upsert(&*pool, &wt.id, platform, &iid.to_string(), &web_url, "open").await?;

        // Hand off to the shared open path so the event carries DB-derived
        // worktrees and repos — first open and resume deliver identical state.
        open_task_impl(&app, &session.id, &task_state, &pool).await?;
        anyhow::Ok(session.id)
    };
    open.await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_explorer(
    short_id: String,
    name: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    store::sessions::rename_explorer(&*pool, &short_id, &name)
        .await
        .map_err(|e| e.to_string())
}

/// Discard a throwaway session: worktree directories, then the session row —
/// every DB child goes with it through the cascades.
#[tauri::command]
pub async fn discard_explorer(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let kind = store::sessions::kind_of(&*pool, &short_id)
        .await
        .map_err(|e| e.to_string())?;
    if !matches!(kind, Some(SessionKind::Explorer) | Some(SessionKind::Review)) {
        return Err(format!("{short_id} is not an explorer or review session"));
    }

    crate::worktrees::cleanup_session_worktrees(&short_id, &pool)
        .await
        .map_err(|e| e.to_string())?;
    store::sessions::remove(&*pool, &short_id)
        .await
        .map_err(|e| e.to_string())?;

    if task_state.get_active_task_id().as_deref() == Some(short_id.as_str()) {
        task_state.set_active_task_id(None);
    }
    app.emit(crate::core::events::EXPLORER_DISCARDED, serde_json::json!({ "short_id": short_id }))
        .map_err(|e| e.to_string())?;
    Ok(())
}
