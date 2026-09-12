use std::sync::{Arc, RwLock};

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::config::{self, ConfigView};
use crate::core::db::models::{Session, SessionKind, Worktree};
use crate::core::db::store;
use crate::core::error::AppResult;

// ─── Module state ─────────────────────────────────────────────────────────────

/// The focused session id, read by MCP tools with no binding of their own.
#[derive(Clone)]
pub struct State {
    inner: Arc<RwLock<Option<String>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_active_task_id(&self) -> Option<String> {
        self.inner.read().ok().and_then(|g| g.clone())
    }

    pub fn set_active_task_id(&self, id: Option<String>) {
        if let Ok(mut g) = self.inner.write() {
            *g = id;
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// The intent of an `open_task_impl` call. Only `Focus` moves the active-task pointer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Open {
    /// Navigate to the session.
    Focus,
    /// Refresh the session in place.
    Refresh,
}

impl Open {
    fn focuses(self) -> bool {
        self == Open::Focus
    }
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config() -> AppResult<Option<ConfigView>> {
    // Do not return the raw Config: it holds a source token.
    Ok(config::get().map(ConfigView::from))
}

#[tauri::command]
pub async fn set_font_size(font_size: u8) -> AppResult<()> {
    config::update(|cfg| cfg.ui.font_size = font_size)?;
    Ok(())
}

#[tauri::command]
pub async fn set_font_family(font_family: String) -> AppResult<()> {
    config::update(|cfg| cfg.ui.font_family = font_family)?;
    Ok(())
}

#[tauri::command]
pub async fn set_agent_font_family(agent_font_family: String) -> AppResult<()> {
    config::update(|cfg| cfg.ui.agent_font_family = agent_font_family)?;
    Ok(())
}

#[tauri::command]
pub async fn set_suggest_actions(suggest_actions: bool) -> AppResult<()> {
    config::update(|cfg| cfg.ui.suggest_actions = suggest_actions)?;
    Ok(())
}

#[tauri::command]
pub async fn set_theme(theme: String) -> AppResult<()> {
    config::update(|cfg| cfg.ui.theme = theme)?;
    Ok(())
}

/// Font families known to fontconfig, for the Settings picker; empty without fontconfig.
#[tauri::command]
pub async fn list_fonts() -> AppResult<Vec<String>> {
    let out = tokio::process::Command::new("fc-list")
        .args([":", "family", "-f", "%{family}\\n"])
        .output()
        .await;

    let Ok(out) = out else { return Ok(vec![]) };
    let mut families: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        // fontconfig lists aliases comma-separated.
        .flat_map(|line| line.split(','))
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();
    families.sort_by_key(|f| f.to_lowercase());
    families.dedup();
    Ok(families)
}

#[tauri::command]
pub async fn open_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    open_task_impl(&app, &short_id, &task_state, &pool, Open::Focus).await?;
    Ok(())
}

/// Point the backend at the focused session; `None` when the last session closes.
#[tauri::command]
pub async fn set_active_task(
    short_id: Option<String>,
    task_state: tauri::State<'_, State>,
) -> AppResult<()> {
    task_state.set_active_task_id(short_id.filter(|s| !s.is_empty()));
    Ok(())
}

/// Open a session and emit `workspace_ready`, or `workspace_stub` for a task with no worktrees.
#[tracing::instrument(skip_all, fields(session = short_id))]
pub(super) async fn open_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
    open: Open,
) -> anyhow::Result<Session> {
    let session = match store::sessions::get_opt(pool, short_id).await? {
        Some(session) => session,
        None => store::sessions::open_task(pool, short_id).await?,
    };

    // A refresh must not move the active-task pointer.
    if open.focuses() {
        task_state.set_active_task_id(Some(session.id.clone()));
    }

    prune_missing_worktrees(&session.id, pool).await?;

    let task = store::sessions::view(pool, &session.id).await?;
    let worktrees = store::worktrees::for_session(pool, &session.id).await?;

    if worktrees.is_empty() && session.kind == SessionKind::Task {
        app.emit(
            crate::core::events::WORKSPACE_STUB,
            serde_json::json!({ "task": task, "kind": session.kind, "focus": open.focuses() }),
        )?;
    } else {
        let repos = store::repos::attached_to(pool, &session.id).await?;
        app.emit(
            crate::core::events::WORKSPACE_READY,
            serde_json::json!({
                "task": task,
                "worktrees": worktrees,
                "repos": repos,
                "kind": session.kind,
                "focus": open.focuses(),
            }),
        )?;
    }

    Ok(session)
}

/// Drop worktrees whose directories are gone and prune their git registration.
async fn prune_missing_worktrees(session_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    let worktrees: Vec<Worktree> = store::worktrees::for_session(pool, session_id).await?;
    for wt in worktrees {
        if std::path::Path::new(&wt.path).exists() {
            continue;
        }
        store::worktrees::close(pool, &wt.id).await?;
        if let Some(repo) = store::repos::get_opt(pool, &wt.repo_id).await? {
            let _ = crate::core::git::output(&repo.local_path, &["worktree", "prune"]).await;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn finish_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    Ok(finish_task_impl(&app, &short_id, &task_state, &pool).await?)
}

/// The confirmation-bridge form of `finish_task`.
pub async fn finish_task_from_payload(
    payload: serde_json::Value,
    pool: &SqlitePool,
    handle: &tauri::AppHandle,
) -> anyhow::Result<()> {
    use tauri::Manager;
    let short_id = payload["task_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("task_id is required"))?;
    finish_task_impl(handle, short_id, &handle.state::<State>(), pool).await
}

#[tracing::instrument(skip_all, fields(session = short_id))]
async fn finish_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    // Set the status at the source before any local teardown.
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    let done_status = provider
        .set_status(&key, crate::provider::types::StatusIntent::Done)
        .await?;

    store::provider_tasks::set_status(pool, short_id, &done_status).await?;
    tear_down_session(app, short_id, &done_status, task_state, pool).await
}

/// Discard a task at its source and tear the session down locally.
#[tauri::command]
pub async fn delete_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    Ok(delete_task_impl(&app, &short_id, &task_state, &pool).await?)
}

#[tracing::instrument(skip_all, fields(session = short_id))]
async fn delete_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    // Discard at the source before any local teardown.
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    provider.discard(&key).await?;

    tear_down_session(app, short_id, "Done", task_state, pool).await
}

/// Close a session locally: worktree directories, rows, the active pointer, and the close event.
async fn tear_down_session(
    app: &tauri::AppHandle,
    short_id: &str,
    done_status: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    crate::worktrees::cleanup_session_worktrees(short_id, pool).await?;
    store::sessions::remove(pool, short_id).await?;

    if task_state.get_active_task_id().as_deref() == Some(short_id) {
        task_state.set_active_task_id(None);
    }

    app.emit(
        crate::core::events::TASK_FINISHED,
        serde_json::json!({ "short_id": short_id, "done_status": done_status }),
    )?;

    Ok(())
}

/// Close the session in the UI and keep its worktrees.
#[tauri::command]
pub async fn pause_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
) -> AppResult<()> {
    task_state.set_active_task_id(None);

    app.emit(
        crate::core::events::TASK_PAUSED,
        serde_json::json!({ "short_id": short_id }),
    )?;

    Ok(())
}

#[tauri::command]
pub async fn set_task_repos(
    short_id: String,
    repo_ids: Vec<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<()> {
    Ok(store::repos::set_attached(&*pool, &short_id, &repo_ids).await?)
}
