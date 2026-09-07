//! Task-less sessions: explorers and MR reviews. `sessions.kind` is the discriminator.

use sqlx::SqlitePool;
use tauri::Emitter;

use super::commands::{open_task_impl, Open};
use super::State;
use crate::core::db::models::{Repo, SessionKind};
use crate::core::db::store;
use crate::core::error::{AppError, AppResult};

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
) -> AppResult<String> {
    let short_id = new_explorer_id();
    let title = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("Explorer {}", &short_id["explorer-".len()..]));

    let session = store::sessions::create_explorer(&*pool, &short_id, &title).await?;
    let task = store::sessions::view(&*pool, &session.id).await?;

    task_state.set_active_task_id(Some(short_id.clone()));

    app.emit(
        crate::core::events::WORKSPACE_READY,
        serde_json::json!({
            "task": task,
            "worktrees": [],
            "repos": [],
            "kind": SessionKind::Explorer,
        }),
    )?;

    Ok(short_id)
}

// ─── Review sessions ──────────────────────────────────────────────────────────
// A review session checks out the MR's source branch with its target as the
// diff base. `(project, iid)` is its identity; reopening the MR resumes it.

fn review_session_id(project_full: &str, iid: u64) -> String {
    format!("review-{}-{iid}", project_full.replace('/', "-"))
}

/// Register the MR's MAIN clone under `<host>/<project_full>` and attach it to the session.
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
) -> AppResult<String> {
    // First open and resume run the same idempotent provisioning.
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

        // Bind the MR; the forge is read from the URL.
        let platform = if web_url.contains("github") {
            "github"
        } else {
            "gitlab"
        };
        store::mrs::upsert(&*pool, &wt.id, platform, &iid.to_string(), &web_url, "open").await?;

        open_task_impl(&app, &session.id, &task_state, &pool, Open::Focus).await?;
        anyhow::Ok(session.id)
    };
    Ok(open.await?)
}

#[tauri::command]
pub async fn rename_explorer(
    short_id: String,
    name: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    Ok(store::sessions::rename_explorer(&*pool, &short_id, &name).await?)
}

/// Discard an explorer or review session: worktree directories, then the session row.
#[tauri::command]
pub async fn discard_explorer(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    let kind = store::sessions::kind_of(&*pool, &short_id).await?;
    if !matches!(
        kind,
        Some(SessionKind::Explorer) | Some(SessionKind::Review)
    ) {
        return Err(AppError::conflict(format!(
            "{short_id} is not an explorer or review session"
        )));
    }

    crate::worktrees::cleanup_session_worktrees(&short_id, &pool).await?;
    store::sessions::remove(&*pool, &short_id).await?;

    if task_state.get_active_task_id().as_deref() == Some(short_id.as_str()) {
        task_state.set_active_task_id(None);
    }
    app.emit(
        crate::core::events::EXPLORER_DISCARDED,
        serde_json::json!({ "short_id": short_id }),
    )?;
    Ok(())
}
