use std::sync::{Arc, RwLock};

use sqlx::SqlitePool;
use tauri::{Emitter, Manager};

use crate::db::schema::{Repo, Task};
use super::config::{Config, ConfigView, load_config_from_dir, save_config_to_dir};
use super::notion::{
    current_sprint_ids, get_task_body_impl, notion_patch, notion_post, page_to_task,
    parse_short_id_number, upsert_task,
};

// ─── Module state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct State {
    inner: Arc<StateInner>,
}

struct StateInner {
    config: RwLock<Option<Config>>,
    active_task_id: RwLock<Option<String>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                config: RwLock::new(None),
                active_task_id: RwLock::new(None),
            }),
        }
    }

    pub fn get_active_task_id(&self) -> Option<String> {
        self.inner
            .active_task_id
            .read()
            .ok()
            .and_then(|g| g.clone())
    }

    pub fn set_active_task_id(&self, id: Option<String>) {
        if let Ok(mut g) = self.inner.active_task_id.write() {
            *g = id;
        }
    }

    pub fn get_config(&self) -> Option<Config> {
        self.inner.config.read().ok().and_then(|g| g.clone())
    }

    pub(super) fn set_config(&self, cfg: Config) {
        if let Ok(mut g) = GLOBAL_CONFIG.write() {
            *g = Some(cfg.clone());
        }
        if let Ok(mut g) = self.inner.config.write() {
            *g = Some(cfg);
        }
    }
}

// Mirror of the managed State's config, readable from modules that have no
// AppHandle/State access (e.g. git_engine::resolve_worktree_root).
static GLOBAL_CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// Last-loaded app config, if any. Kept in sync by every `set_config`.
pub fn global_config() -> Option<Config> {
    GLOBAL_CONFIG.read().ok().and_then(|g| g.clone())
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub fn ensure_config(app: &tauri::AppHandle, state: &State) -> anyhow::Result<Config> {
    if let Some(cfg) = state.get_config() {
        return Ok(cfg);
    }
    let cfg_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| anyhow::anyhow!("cannot get config dir: {e}"))?;
    let cfg = load_config_from_dir(&cfg_dir)?;
    state.set_config(cfg.clone());
    Ok(cfg)
}

/// Derive the git branch name for a task (lower-case the full ID).
pub fn derive_branch(short_id: &str) -> String {
    short_id.to_lowercase()
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config(
    app: tauri::AppHandle,
    task_state: tauri::State<'_, State>,
) -> Result<Option<ConfigView>, String> {
    if task_state.get_config().is_none() {
        if let Ok(cfg_dir) = app.path().app_config_dir() {
            match load_config_from_dir(&cfg_dir) {
                Ok(cfg) => task_state.set_config(cfg),
                // A corrupt config must be distinguishable from "not configured".
                Err(e) => tracing::warn!("config not loaded: {e}"),
            }
        }
    }
    // A view, not the Config: the Notion token stays in Rust.
    Ok(task_state.get_config().map(ConfigView::from))
}

/// Change one UI preference, persist it, and publish it. Every `set_*` below is
/// this and nothing else, so the read-modify-save-publish order lives in one place.
fn save_ui(
    app: &tauri::AppHandle,
    task_state: &State,
    edit: impl FnOnce(&mut super::config::UiConfig),
) -> Result<(), String> {
    let mut cfg = ensure_config(app, task_state).map_err(|e| e.to_string())?;
    edit(&mut cfg.ui);
    let cfg_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    save_config_to_dir(&cfg_dir, &cfg).map_err(|e| e.to_string())?;
    task_state.set_config(cfg);
    Ok(())
}

#[tauri::command]
pub async fn set_font_size(
    font_size: u8,
    app: tauri::AppHandle,
    task_state: tauri::State<'_, State>,
) -> Result<(), String> {
    save_ui(&app, &task_state, |ui| ui.font_size = font_size)
}

#[tauri::command]
pub async fn set_font_family(
    font_family: String,
    app: tauri::AppHandle,
    task_state: tauri::State<'_, State>,
) -> Result<(), String> {
    save_ui(&app, &task_state, |ui| ui.font_family = font_family)
}

#[tauri::command]
pub async fn set_theme(
    theme: String,
    app: tauri::AppHandle,
    task_state: tauri::State<'_, State>,
) -> Result<(), String> {
    save_ui(&app, &task_state, |ui| ui.theme = theme)
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
pub async fn list_tasks(
    app: tauri::AppHandle,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Task>, String> {
    list_tasks_impl(&app, &task_state, &pool)
        .await
        .map_err(|e| e.to_string())
}

async fn list_tasks_impl(
    app: &tauri::AppHandle,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Task>> {
    let cfg = ensure_config(app, task_state)?;

    let mut filter_conditions: Vec<serde_json::Value> = vec![];

    if cfg.notion.filters.filter_by_assignee {
        filter_conditions.push(serde_json::json!({
            "property": cfg.notion.properties.assignee
                .as_deref()
                .unwrap_or("Assignee"),
            "people": { "contains": cfg.notion.user_id }
        }));
    }

    for status in &cfg.notion.filters.exclude_statuses {
        filter_conditions.push(serde_json::json!({
            "property": cfg.notion.properties.status,
            "status": { "does_not_equal": status }
        }));
    }

    // Sprint filtering only applies when a sprint property is configured —
    // filtering on a property the database doesn't have 400s the whole query.
    if let Some(sprint_prop) = cfg.notion.properties.sprint.as_deref() {
        let sprint_db =
            super::notion::sprint_database_id(&cfg.notion.token, &cfg.notion.database_id, sprint_prop)
                .await;
        let sprint_ids = match &sprint_db {
            Some(db) => current_sprint_ids(&cfg.notion.token, db).await,
            None => vec![],
        };
        if !sprint_ids.is_empty() {
            let sprint_filters: Vec<serde_json::Value> = sprint_ids
                .iter()
                .map(|id| serde_json::json!({
                    "property": sprint_prop,
                    "relation": { "contains": id }
                }))
                .collect();
            if sprint_filters.len() == 1 {
                filter_conditions.push(sprint_filters.into_iter().next().unwrap());
            } else {
                filter_conditions.push(serde_json::json!({ "or": sprint_filters }));
            }
        }
    }

    let body = if filter_conditions.len() > 1 {
        serde_json::json!({ "filter": { "and": filter_conditions } })
    } else if filter_conditions.len() == 1 {
        serde_json::json!({ "filter": filter_conditions.remove(0) })
    } else {
        serde_json::json!({})
    };

    let resp = notion_post(
        &cfg.notion.token,
        &format!("v1/databases/{}/query", cfg.notion.database_id),
        &body,
    ).await?;

    let mut tasks = vec![];
    if let Some(results) = resp["results"].as_array() {
        for page in results {
            match page_to_task(page, &cfg.notion) {
                Ok(task) => {
                    upsert_task(&task, pool).await?;
                    tasks.push(task);
                }
                Err(e) => tracing::warn!("skipping page: {e}"),
            }
        }
    }

    Ok(tasks)
}

#[tauri::command]
pub async fn open_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    open_task_impl(&app, &short_id, &task_state, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Point the backend at the session the user is looking at. EVERY MCP tool
/// resolves its target from `active_task_id` (get_active_task, get_worktrees,
/// get_task_diff, annotations, the push guard, create_task_from_explorer), so
/// without this the agent operates on whichever task was last *opened* rather
/// than the focused one. The frontend calls it whenever the active session (or
/// its task id) changes; `None` when the last session closes.
#[tauri::command]
pub async fn set_active_task(
    short_id: Option<String>,
    task_state: tauri::State<'_, State>,
) -> Result<(), String> {
    task_state.set_active_task_id(short_id.filter(|s| !s.is_empty()));
    Ok(())
}

pub(super) async fn open_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let task = crate::db::load::task_opt(pool, short_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Task {short_id} not in local DB — sync tasks first"))?;

    task_state.set_active_task_id(Some(task.short_id.clone()));

    // Re-activate worktrees deactivated by pause_task — everything (diff,
    // commits, agent, MCP) filters on is_active = 1, so without this a
    // paused task reopens as an empty workspace.
    sqlx::query("UPDATE worktrees SET is_active = 1 WHERE task_id = ?")
        .bind(&task.short_id)
        .execute(pool)
        .await?;

    let worktrees = crate::db::load::all_worktrees(pool, &task.short_id).await?;

    // Prune worktrees whose directories were manually deleted
    for wt in &worktrees {
        if !std::path::Path::new(&wt.path).exists() {
            sqlx::query("DELETE FROM mrs WHERE worktree_id = ?")
                .bind(&wt.id)
                .execute(pool)
                .await?;
            sqlx::query("DELETE FROM worktrees WHERE id = ?")
                .bind(&wt.id)
                .execute(pool)
                .await?;
            let remaining: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM worktrees WHERE task_id = ? AND repo_id = ?",
            )
            .bind(&task.short_id)
            .bind(&wt.repo_id)
            .fetch_one(pool)
            .await?;
            if remaining == 0 {
                sqlx::query("DELETE FROM task_repos WHERE task_id = ? AND repo_id = ?")
                    .bind(&task.short_id)
                    .bind(&wt.repo_id)
                    .execute(pool)
                    .await?;
            }
            // Clear git's stale worktree registration in the source repo, or
            // re-provisioning the same branch fails with "already registered".
            let repo = crate::db::load::repo_opt(pool, &wt.repo_id).await?;
            if let Some(repo) = repo {
                let local = repo.local_path.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("git")
                        .args(["-C", &local, "worktree", "prune"])
                        .output()
                })
                .await;
            }
        }
    }

    // Re-fetch after pruning
    let worktrees = crate::db::load::all_worktrees(pool, &task.short_id).await?;

    // Synthetic sessions (no Notion page) are identified by an empty page id;
    // they never go through the task-open wizard, even with no worktrees yet.
    // Review sessions are the synthetic ones with a review- short id.
    let is_synthetic = task.notion_page_id.is_empty();
    let kind = if task.short_id == super::DESK_ID {
        "desk"
    } else if task.short_id.starts_with("review-") {
        "review"
    } else if is_synthetic {
        "explorer"
    } else {
        "task"
    };

    if worktrees.is_empty() && !is_synthetic {
        app.emit(crate::events::WORKSPACE_STUB, serde_json::json!({ "task": task, "kind": kind }))?;
    } else {
        let repos: Vec<Repo> =
            sqlx::query_as(
                "SELECT r.* FROM repos r JOIN task_repos tr ON r.id = tr.repo_id WHERE tr.task_id = ?",
            )
            .bind(&task.short_id)
            .fetch_all(pool)
            .await?;

        app.emit(
            crate::events::WORKSPACE_READY,
            serde_json::json!({ "task": task, "worktrees": worktrees, "repos": repos, "kind": kind }),
        )?;
    }

    Ok(())
}

pub(super) async fn delete_task_rows(task_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM mrs WHERE worktree_id IN (SELECT id FROM worktrees WHERE task_id = ?)")
        .bind(task_id)
        .execute(&mut *tx)
        .await?;
    for table in ["worktrees", "task_repos", "annotations", "tab_snapshots", "agent_sessions", "pending_confirmations", "reviewed_files", "task_time"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE task_id = ?"))
            .bind(task_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
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

async fn finish_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let cfg = ensure_config(app, task_state)?;

    // Look up the task and mark it done in Notion BEFORE any destructive
    // teardown — if either fails, the workspace is still intact.
    let task = crate::db::load::task(pool, short_id).await?;
    if task.notion_page_id.is_empty() {
        return Err(anyhow::anyhow!(
            "{short_id} is an explorer session — discard it instead of finishing"
        ));
    }

    let done_status = &cfg.notion.status_map.done;
    let prop_name = &cfg.notion.properties.status;
    notion_patch(
        &cfg.notion.token,
        &format!("v1/pages/{}", task.notion_page_id),
        &serde_json::json!({
            "properties": {
                prop_name: { "status": { "name": done_status } }
            }
        }),
    )
    .await?;

    crate::git_engine::cleanup_task_worktrees(short_id, pool).await?;
    delete_task_rows(short_id, pool).await?;

    sqlx::query("UPDATE tasks SET status = ? WHERE short_id = ?")
        .bind(done_status)
        .bind(short_id)
        .execute(pool)
        .await?;

    if task_state.get_active_task_id().as_deref() == Some(short_id) {
        task_state.set_active_task_id(None);
    }

    app.emit(
        crate::events::TASK_FINISHED,
        serde_json::json!({ "short_id": short_id, "done_status": done_status }),
    )?;

    Ok(())
}

#[tauri::command]
pub async fn pause_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    task_state.set_active_task_id(None);

    sqlx::query("UPDATE worktrees SET is_active = 0 WHERE task_id = ?")
        .bind(&short_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    app.emit(crate::events::TASK_PAUSED, serde_json::json!({ "short_id": short_id }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn sync_task(
    app: tauri::AppHandle,
    short_id: String,
    task_state: tauri::State<'_, State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Task, String> {
    let cfg = ensure_config(&app, &task_state).map_err(|e| e.to_string())?;

    let num = parse_short_id_number(&short_id)
        .ok_or_else(|| format!("Cannot parse short_id: {short_id}"))?;

    let body = serde_json::json!({
        "filter": {
            "property": "unique_id",
            "unique_id": { "equals": num }
        }
    });
    let resp = notion_post(
        &cfg.notion.token,
        &format!("v1/databases/{}/query", cfg.notion.database_id),
        &body,
    )
    .await
    .map_err(|e| e.to_string())?;

    let page = resp["results"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("Task {short_id} not found in Notion"))?;

    let task = page_to_task(page, &cfg.notion).map_err(|e| e.to_string())?;
    upsert_task(&task, &pool).await.map_err(|e| e.to_string())?;

    Ok(task)
}

#[tauri::command]
pub async fn set_task_repos(
    short_id: String,
    repo_ids: Vec<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let now = chrono::Utc::now().timestamp();

    sqlx::query("DELETE FROM task_repos WHERE task_id = ?")
        .bind(&short_id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;

    for repo_id in &repo_ids {
        sqlx::query(
            "INSERT INTO task_repos (task_id, repo_id, added_at) VALUES (?, ?, ?)",
        )
        .bind(&short_id)
        .bind(repo_id)
        .bind(now)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Called by the confirmation bridge when a notion.status op is approved.
/// `token`/`status_prop_name` are injected by `execute_op` from config —
/// secrets never live in the persisted confirmation payload.
pub async fn update_notion_status_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let token = payload["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token in notion.status payload"))?;
    let page_id = payload["notion_page_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing notion_page_id"))?;
    let status = payload["status"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing status"))?;
    let prop_name = payload["status_prop_name"].as_str().unwrap_or("Status");

    let body = serde_json::json!({
        "properties": {
            prop_name: {
                "status": { "name": status }
            }
        }
    });

    notion_patch(token, &format!("v1/pages/{page_id}"), &body).await?;
    Ok(())
}

/// The ticket body as markdown — what the overview renders. Going through
/// markdown (rather than a bespoke Notion-block renderer) means one renderer for
/// task and MR descriptions, Notion's inline annotations survive, AND markdown
/// typed literally into Notion (`**bold**`, backticks) displays as intended.
#[tauri::command]
pub async fn get_task_body_markdown(
    notion_page_id: String,
    task_state: tauri::State<'_, State>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let cfg = ensure_config(&app, &task_state).map_err(|e| e.to_string())?;
    let blocks = get_task_body_impl(&notion_page_id, &cfg.notion.token)
        .await
        .map_err(|e| e.to_string())?;
    Ok(super::notion::blocks_to_markdown(&blocks))
}
