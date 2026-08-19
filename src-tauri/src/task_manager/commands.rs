use std::sync::{Arc, RwLock};

use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::config::{self, Config, ConfigView};
use crate::core::db::models::{Session, SessionKind, TaskView, Worktree};
use crate::core::db::store;
use super::notion::{
    current_sprint_ids, get_task_body_impl, notion_patch, notion_post, page_to_task,
    parse_short_id_number,
};

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

// ─── IPC commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config() -> Result<Option<ConfigView>, String> {
    // A view, not the Config: the Notion token stays in Rust.
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
pub async fn list_tasks(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<TaskView>, String> {
    list_tasks_impl(&pool).await.map_err(|e| e.to_string())
}

/// One page of the tasks-database query, with the filters from config applied.
async fn query_task_pages(
    cfg: &Config,
    cursor: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
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

    let mut body = if filter_conditions.len() > 1 {
        serde_json::json!({ "filter": { "and": filter_conditions } })
    } else if filter_conditions.len() == 1 {
        serde_json::json!({ "filter": filter_conditions.remove(0) })
    } else {
        serde_json::json!({})
    };
    body["page_size"] = serde_json::json!(100);
    if let Some(cursor) = cursor {
        body["start_cursor"] = serde_json::json!(cursor);
    }

    notion_post(
        &cfg.notion.token,
        &format!("v1/databases/{}/query", cfg.notion.database_id),
        &body,
    )
    .await
}

/// A runaway-pagination backstop far above any real queue.
const MAX_TASK_PAGES: usize = 30;

async fn list_tasks_impl(pool: &SqlitePool) -> anyhow::Result<Vec<TaskView>> {
    let cfg = config::require()?;

    let mut tasks: Vec<TaskView> = vec![];
    let mut cursor: Option<String> = None;

    for _ in 0..MAX_TASK_PAGES {
        let resp = query_task_pages(&cfg, cursor.as_deref()).await?;

        if let Some(results) = resp["results"].as_array() {
            for page in results {
                match page_to_task(page, &cfg.notion) {
                    Ok(task) => {
                        store::notion_tasks::upsert(pool, &task).await?;
                        tasks.push(task.into());
                    }
                    Err(e) => tracing::warn!("skipping page: {e}"),
                }
            }
        }

        if resp["has_more"].as_bool() != Some(true) {
            break;
        }
        match resp["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_string()),
            None => break,
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
pub(super) async fn open_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<Session> {
    let session = match store::sessions::get_opt(pool, short_id).await? {
        Some(session) => session,
        None => store::sessions::open_task(pool, short_id).await?,
    };

    task_state.set_active_task_id(Some(session.id.clone()));

    prune_missing_worktrees(&session.id, pool).await?;

    let task = store::sessions::view(pool, &session.id).await?;
    let worktrees = store::worktrees::for_session(pool, &session.id).await?;

    if worktrees.is_empty() && session.kind == SessionKind::Task {
        app.emit(
            crate::core::events::WORKSPACE_STUB,
            serde_json::json!({ "task": task, "kind": session.kind }),
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

async fn finish_task_impl(
    app: &tauri::AppHandle,
    short_id: &str,
    task_state: &State,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let cfg = config::require()?;

    // Mark the task done in Notion BEFORE any destructive teardown — if it
    // fails, the workspace is still intact.
    let page_id = task_page_id(pool, short_id).await?;
    let done_status = cfg.notion.status_map.done.clone();
    let prop_name = &cfg.notion.properties.status;
    notion_patch(
        &cfg.notion.token,
        &format!("v1/pages/{page_id}"),
        &serde_json::json!({
            "properties": {
                prop_name: { "status": { "name": done_status } }
            }
        }),
    )
    .await?;

    store::notion_tasks::set_status(pool, short_id, &done_status).await?;
    tear_down_session(app, short_id, &done_status, task_state, pool).await
}

/// A task the user is deleting outright, not finishing.
///
/// Notion has no hard delete through the API: a page goes to the workspace's
/// trash, where Notion keeps it for thirty days. The local teardown is the same
/// one `finish_task` runs.
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
    let cfg = config::require()?;

    // Trash the page BEFORE any local teardown, the same order finish_task uses.
    let page_id = task_page_id(pool, short_id).await?;
    notion_patch(
        &cfg.notion.token,
        &format!("v1/pages/{page_id}"),
        &serde_json::json!({ "in_trash": true }),
    )
    .await?;

    tear_down_session(app, short_id, &cfg.notion.status_map.done, task_state, pool).await
}

/// The Notion page behind a task session. Explorers and reviews have none and
/// are discarded, not finished.
async fn task_page_id(pool: &SqlitePool, short_id: &str) -> anyhow::Result<String> {
    let session = store::sessions::get(pool, short_id).await?;
    session.notion_page_id.ok_or_else(|| {
        anyhow::anyhow!("{short_id} is a {:?} session — discard it instead", session.kind)
    })
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
pub async fn sync_task(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TaskView, String> {
    let cfg = config::require().map_err(|e| e.to_string())?;

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
    store::notion_tasks::upsert(&*pool, &task).await.map_err(|e| e.to_string())?;

    Ok(task.into())
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
pub async fn get_task_body_markdown(notion_page_id: String) -> Result<String, String> {
    let cfg = config::require().map_err(|e| e.to_string())?;
    let blocks = get_task_body_impl(&notion_page_id, &cfg.notion.token)
        .await
        .map_err(|e| e.to_string())?;
    Ok(super::notion::blocks_to_markdown(&blocks))
}
