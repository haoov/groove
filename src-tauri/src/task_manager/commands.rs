use std::sync::{Arc, RwLock};

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::config::{self, ConfigView};
use crate::core::db::models::{Session, SessionKind, Worktree};
use crate::core::db::store;

// ─── Module state ─────────────────────────────────────────────────────────────

/// The focused session, for MCP tools with no binding of their own. Config
/// lives in `core::config`, not here.
#[derive(Clone)]
pub struct State {
    inner: Arc<RwLock<Option<String>>>,
}

impl State {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(None)) }
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

/// What an `open_task_impl` call MEANS, since the two callers differ: the user
/// asking for a session, or a landed write pushing new rows into one already on
/// screen. Only a focusing open may move the user or the active-task pointer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Open {
    /// The user asked for this session — navigate to it.
    Focus,
    /// Repos or worktrees changed — refresh in place, do not navigate.
    Refresh,
}

impl Open {
    fn focuses(self) -> bool {
        self == Open::Focus
    }
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config() -> Result<Option<ConfigView>, String> {
    // A view, not the Config: a source token stays in Rust.
    Ok(config::get().map(ConfigView::from))
}

#[tauri::command]
pub async fn set_font_size(font_size: u8) -> Result<(), String> {
    config::update(|cfg| cfg.ui.font_size = font_size)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_font_family(font_family: String) -> Result<(), String> {
    config::update(|cfg| cfg.ui.font_family = font_family)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_agent_font_family(agent_font_family: String) -> Result<(), String> {
    config::update(|cfg| cfg.ui.agent_font_family = agent_font_family)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_suggest_actions(suggest_actions: bool) -> Result<(), String> {
    config::update(|cfg| cfg.ui.suggest_actions = suggest_actions)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_theme(theme: String) -> Result<(), String> {
    config::update(|cfg| cfg.ui.theme = theme)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Monospace families fontconfig knows about, for the Settings picker.
///
/// Reading the real list is the point: the font that started this was set to a
/// name nothing matched, and CSS fails silently to the next family. An empty
/// result (no fontconfig) is not an error — Settings falls back to a text field.
#[tauri::command]
pub async fn list_fonts() -> Result<Vec<String>, String> {
    let out = tokio::process::Command::new("fc-list")
        .args([":", "family", "-f", "%{family}\\n"])
        .output()
        .await;

    let Ok(out) = out else { return Ok(vec![]) };
    let mut families: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        // fontconfig reports aliases comma-separated ("JetBrainsMono NFM,Regular").
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
) -> Result<(), String> {
    open_task_impl(&app, &short_id, &task_state, &pool, Open::Focus)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Point the backend at the session the user is looking at. EVERY MCP tool
/// resolves its target from the active session, so without this the agent
/// operates on whichever session was last *opened* rather than the focused one.
/// The frontend calls it whenever the active session changes; `None` when the
/// last session closes.
#[tauri::command]
pub async fn set_active_task(
    short_id: Option<String>,
    task_state: tauri::State<'_, State>,
) -> Result<(), String> {
    task_state.set_active_task_id(short_id.filter(|s| !s.is_empty()));
    Ok(())
}

/// Open a session: an existing one re-emits its state, a mirrored task gets
/// its session row on first open. Emits `workspace_ready` — or `workspace_stub`
/// for a task that still has no worktrees, which sends the frontend to the
/// repo-picking wizard.
///
/// `open` rides along in the payload as `focus`: the frontend uses the same
/// event to mount a session and to refresh one, and only the caller knows which
/// it asked for.
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

    // A refresh must not steal the pointer: MCP tools with no binding of their
    // own resolve from it, and the user is still looking at another session.
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

/// Drop worktrees whose directories were deleted by hand, and clear git's stale
/// registration in the source repo — re-provisioning the same branch otherwise
/// fails with "already registered".
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
) -> Result<(), String> {
    finish_task_impl(&app, &short_id, &task_state, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// The approval-gated form. Same teardown as the Finish task button.
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

async fn finish_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    // Mark it done at the source BEFORE any destructive teardown — if that
    // fails, the workspace is still intact.
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    // The label the provider ACTUALLY wrote is what the mirror records — the
    // config's guess of it could differ (GitHub picks the column off the board).
    let done_status =
        provider.set_status(&key, crate::provider::types::StatusIntent::Done).await?;

    store::provider_tasks::set_status(pool, short_id, &done_status).await?;
    tear_down_session(app, short_id, &done_status, task_state, pool).await
}

/// A task the user is deleting outright, not finishing.
///
/// What that means at the source is the provider's business — Notion trashes the
/// page, which its workspace keeps for thirty days. The local teardown is the
/// same one `finish_task` runs.
#[tauri::command]
pub async fn delete_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    delete_task_impl(&app, &short_id, &task_state, &pool)
        .await
        .map_err(|e| e.to_string())
}

async fn delete_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    // Discard at the source BEFORE any local teardown, the order finish_task uses.
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    provider.discard(&key).await?;

    // The row is deleted with the session, so the status in the teardown event
    // is cosmetic — a literal beats asking the provider for a label it never wrote.
    tear_down_session(app, short_id, "Done", task_state, pool).await
}

/// Close a session locally: its worktree directories, its rows (one cascading
/// delete), the active pointer, and the event the UI closes the session on.
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

/// Put a session away without finishing it: the UI closes, the worktrees stay.
/// Reopening is a plain `open_task`.
#[tauri::command]
pub async fn pause_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
) -> Result<(), String> {
    task_state.set_active_task_id(None);

    app.emit(crate::core::events::TASK_PAUSED, serde_json::json!({ "short_id": short_id }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn set_task_repos(
    short_id: String,
    repo_ids: Vec<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    store::repos::set_attached(&*pool, &short_id, &repo_ids)
        .await
        .map_err(|e| e.to_string())
}

