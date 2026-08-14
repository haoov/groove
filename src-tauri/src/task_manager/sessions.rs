//! Synthetic, task-less sessions: the desk, explorers and MR reviews.
//!
//! All are local-only rows in `tasks` with an empty `notion_page_id`, which is
//! what lets them reuse the entire task machinery (worktrees, agent, diff,
//! add-repo) without ever appearing in Notion. The `short_id` prefix
//! (`explorer-` / `review-`) is the discriminator everywhere else in the app;
//! the desk is the single fixed id `desk`.

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::db::schema::{Repo, Task};
use super::commands::{delete_task_rows, open_task_impl};
use super::State;

// ─── The desk (one agent, no workspace) ───────────────────────────────────────
// A place to ask questions and file tasks without opening a session for it. It
// needs a `tasks` row only to have an identity: the agent PTY, the MCP `?task=`
// binding and the hook-reported activity all key off a task id. It never gets
// worktrees, never appears in Live or the session tabs, and is not a workspace.

/// The desk's fixed short id. Fixed, not random, so its Claude conversation
/// (a deterministic uuid derived from this id) resumes for the life of the app.
pub const DESK_ID: &str = "desk";

/// Create the desk row if it isn't there, and return it.
///
/// Unlike `open_explorer_session` this does NOT set the active task and does NOT
/// emit `WORKSPACE_READY`: there is no workspace to open, and stealing focus
/// would drag the user off whatever they were looking at.
#[tauri::command]
pub async fn ensure_desk_session(pool: tauri::State<'_, SqlitePool>) -> Result<Task, String> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO tasks (short_id, notion_page_id, title, status, priority, last_synced_at)
         VALUES (?, '', 'Desk', 'in_progress', NULL, ?)
         ON CONFLICT(short_id) DO NOTHING",
    )
    .bind(DESK_ID)
    .bind(now)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    crate::db::load::task(&pool, DESK_ID).await.map_err(|e| e.to_string())
}

// ─── Explorer sessions (task-less workspaces) ─────────────────────────────────
// An explorer is a synthetic, local-only "task" (empty notion_page_id) so it
// reuses all task machinery (worktrees, agent, diff, add-repo) yet never appears
// on the Notion board. It can later be converted into a real task.

fn new_explorer_id() -> String {
    let uid = uuid::Uuid::new_v4().simple().to_string();
    format!("explorer-{}", &uid[..6])
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
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO tasks (short_id, notion_page_id, title, status, priority, last_synced_at)
         VALUES (?, '', ?, 'in_progress', NULL, ?)",
    )
    .bind(&short_id)
    .bind(&title)
    .bind(now)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    task_state.set_active_task_id(Some(short_id.clone()));

    app.emit(
        crate::events::WORKSPACE_READY,
        serde_json::json!({
            "task": {
                "short_id": short_id,
                "notion_page_id": "",
                "title": title,
                "status": "in_progress",
                "priority": null,
                "last_synced_at": now,
            },
            "worktrees": [],
            "repos": [],
            "kind": "explorer",
        }),
    )
    .map_err(|e| e.to_string())?;

    Ok(short_id)
}

// ─── Review sessions ──────────────────────────────────────────────────────────
// A review session is a synthetic task (like an explorer) whose worktree checks
// out an MR's source branch, with the MR's target pinned as the diff/log base.
// Deterministic id: reopening the same MR resumes the session (annotations and
// viewed-file state intact). Finishing reuses discard_explorer.

fn review_session_id(project_full: &str, iid: u64) -> String {
    let key = format!("{project_full}!{iid}");
    let uid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, key.as_bytes())
        .simple()
        .to_string();
    format!("review-{}", &uid[..6])
}

/// Create (or refresh the title of) the synthetic task row backing the review.
async fn upsert_review_task(
    short_id: &str,
    title: &str,
    now: i64,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO tasks (short_id, notion_page_id, title, status, priority, last_synced_at)
         VALUES (?, '', ?, 'in_progress', NULL, ?)
         ON CONFLICT(short_id) DO UPDATE SET title = excluded.title, last_synced_at = excluded.last_synced_at",
    )
    .bind(short_id)
    .bind(title)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Register the MR's MAIN clone and attach it to the session.
async fn attach_review_repo(
    short_id: &str,
    local_path: String,
    now: i64,
    pool: &SqlitePool,
) -> anyhow::Result<Repo> {
    let remote_url = crate::git_engine::run_git(&local_path, &["remote", "get-url", "origin"])
        .await
        .map_err(|e| anyhow::anyhow!("could not read origin URL of {local_path}: {e}"))?
        .trim()
        .to_string();
    let repo = crate::git_engine::register_repo_impl(local_path, remote_url, pool).await?;

    // `added_at` is NOT NULL with no default — omitting it under INSERT OR IGNORE
    // silently dropped this row, so reopening the session found no repos.
    sqlx::query(
        "INSERT INTO task_repos (task_id, repo_id, added_at) VALUES (?, ?, ?)
         ON CONFLICT(task_id, repo_id) DO NOTHING",
    )
    .bind(short_id)
    .bind(&repo.id)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(repo)
}

/// Bind the MR to the worktree so the Forge / MR-overview machinery works from
/// the first render. Idempotent: reopening a review must not duplicate the row.
async fn bind_mr_row(
    worktree_id: &str,
    iid: u64,
    web_url: &str,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM mrs WHERE worktree_id = ? AND remote_id = ?")
            .bind(worktree_id)
            .bind(iid.to_string())
            .fetch_optional(pool)
            .await?;
    if existing.is_some() {
        return Ok(());
    }
    // The forge comes from the URL, not a constant: a GitHub review session used to
    // be stored as 'gitlab', which is what the UI reads to pick `#42` over `!42`.
    let platform = if web_url.contains("github") { "github" } else { "gitlab" };
    sqlx::query(
        "INSERT INTO mrs (id, worktree_id, platform, remote_id, url, state)
         VALUES (?, ?, ?, ?, ?, 'open')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(worktree_id)
    .bind(platform)
    .bind(iid.to_string())
    .bind(web_url)
    .execute(pool)
    .await?;
    Ok(())
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
    let short_id = review_session_id(&project_full, iid);
    let now = chrono::Utc::now().timestamp();

    // Both first open and resume run the same provisioning: every step is
    // idempotent (upserts + a `worktree add` that tolerates "already exists"), and
    // re-running it is what makes closing and reopening a review reliable — the
    // worktree may have been pruned, the MR has probably moved on, and a session
    // created by an older build may be missing rows entirely.
    let open = async {
        upsert_review_task(&short_id, &title, now, &pool).await?;
        let repo = attach_review_repo(&short_id, local_path, now, &pool).await?;
        let wt = crate::git_engine::provision_review_worktree(
            &short_id,
            &repo,
            &source_branch,
            &target_branch,
            &pool,
        )
        .await?;
        bind_mr_row(&wt.id, iid, &web_url, &pool).await?;

        // Hand off to the shared open path so the event carries DB-derived
        // worktrees and repos (rather than the objects we happen to hold here) —
        // first open and resume then deliver byte-identical state, and a missing
        // row can't hide.
        open_task_impl(&app, &short_id, &task_state, &pool).await
    };
    open.await.map_err(|e| e.to_string())?;

    Ok(short_id)
}

/// Row shape for `get_reviewed_files` — which files the reviewer marked viewed.
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ReviewedFile {
    pub repo_id: String,
    pub file_path: String,
}

#[tauri::command]
pub async fn set_file_reviewed(
    task_id: String,
    repo_id: String,
    file_path: String,
    reviewed: bool,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let q = if reviewed {
        sqlx::query(
            "INSERT OR IGNORE INTO reviewed_files (task_id, repo_id, file_path) VALUES (?, ?, ?)",
        )
    } else {
        sqlx::query("DELETE FROM reviewed_files WHERE task_id = ? AND repo_id = ? AND file_path = ?")
    };
    q.bind(&task_id)
        .bind(&repo_id)
        .bind(&file_path)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_reviewed_files(
    task_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<ReviewedFile>, String> {
    sqlx::query_as("SELECT repo_id, file_path FROM reviewed_files WHERE task_id = ?")
        .bind(&task_id)
        .fetch_all(&*pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_explorer(
    short_id: String,
    name: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    sqlx::query("UPDATE tasks SET title = ? WHERE short_id = ? AND notion_page_id = ''")
        .bind(&name)
        .bind(&short_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete every DB row tied to a task, child rows first, in one transaction.
/// Shared by finish_task and discard_explorer so the table list can't drift.
#[tauri::command]
pub async fn discard_explorer(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    // Every query below is scoped to "empty notion_page_id", which the desk also
    // matches — discarding it would delete the row its agent is bound to.
    if short_id == DESK_ID {
        return Err("the desk cannot be discarded".into());
    }
    // Remove worktree directories on disk + prune git refs.
    crate::git_engine::cleanup_task_worktrees(&short_id, &pool)
        .await
        .map_err(|e| e.to_string())?;
    // Then tear down every DB row tied to this synthetic task.
    delete_task_rows(&short_id, &pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM tasks WHERE short_id = ? AND notion_page_id = ''")
        .bind(&short_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    if task_state.get_active_task_id().as_deref() == Some(short_id.as_str()) {
        task_state.set_active_task_id(None);
    }
    app.emit(crate::events::EXPLORER_DISCARDED, serde_json::json!({ "short_id": short_id }))
        .map_err(|e| e.to_string())?;
    Ok(())
}
