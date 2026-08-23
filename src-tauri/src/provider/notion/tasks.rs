//! The task queue: querying the tasks database, syncing one task, and the two
//! status writes the session lifecycle needs.

use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

use sqlx::SqlitePool;

use crate::core::config::{self, Config, NotionConfig};
use crate::core::db::models::{ProviderTask, TaskView};
use crate::core::db::store;

use super::page::{page_to_task, parse_short_id_number};

/// A runaway-pagination backstop far above any real queue.
const MAX_TASK_PAGES: usize = 30;

// ─── Sprint filter ────────────────────────────────────────────────────────────

/// Current-sprint page ids are re-read this often. Same TTL as the schema cache:
/// sprints change per week, not per query, and without this every task listing
/// pays two extra Notion round-trips before the real query starts.
const SPRINT_TTL_SECS: i64 = 300;

type SprintCache = RwLock<HashMap<String, (i64, Vec<String>)>>;

fn sprint_cache() -> &'static SprintCache {
    static CACHE: OnceLock<SprintCache> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// The sprint database, read off the Sprint relation rather than configured: the
/// property already says where it points. `None` when the task database has no
/// sprint relation, in which case there is nothing to filter by.
async fn sprint_database_id(token: &str, database_id: &str, sprint_prop: &str) -> Option<String> {
    super::schema::load(token, database_id)
        .await
        .ok()?
        .relation_target(sprint_prop)
        .map(str::to_string)
}

/// Page ids of the sprint database's rows whose status property says "Current",
/// cached. The property is found by TYPE in the sprint database's own schema.
pub(super) async fn current_sprint_ids(token: &str, sprint_db_id: &str) -> Vec<String> {
    let now = chrono::Utc::now().timestamp();
    if let Ok(map) = sprint_cache().read() {
        if let Some((at, ids)) = map.get(sprint_db_id) {
            if now - at < SPRINT_TTL_SECS {
                return ids.clone();
            }
        }
    }

    let ids = fetch_current_sprint_ids(token, sprint_db_id).await;
    if let Ok(mut map) = sprint_cache().write() {
        map.insert(sprint_db_id.to_string(), (now, ids.clone()));
    }
    ids
}

async fn fetch_current_sprint_ids(token: &str, sprint_db_id: &str) -> Vec<String> {
    let Some(status_prop) = status_property_of(token, sprint_db_id).await else {
        tracing::warn!("[sprint filter] sprint DB {sprint_db_id} has no status property");
        return vec![];
    };
    let body = serde_json::json!({
        "filter": {
            "property": status_prop,
            "status": { "equals": "Current" }
        }
    });
    match super::api::paginate_post(token, &format!("v1/databases/{sprint_db_id}/query"), &body, 2).await {
        Ok(rows) => rows
            .iter()
            .filter_map(|p| p["id"].as_str().map(str::to_string))
            .collect(),
        Err(e) => {
            tracing::warn!("[sprint filter] sprint DB query failed — check integration permissions: {e}");
            vec![]
        }
    }
}

/// A database's status property name, by type.
async fn status_property_of(token: &str, database_id: &str) -> Option<String> {
    super::schema::load(token, database_id)
        .await
        .ok()?
        .properties
        .iter()
        .find(|p| p.kind == "status")
        .map(|p| p.name.clone())
}

// ─── Queue query ──────────────────────────────────────────────────────────────

/// The filter the config describes, as a Notion query body.
async fn queue_filter(cfg: &Config) -> serde_json::Value {
    let n = &cfg.notion;
    let mut conditions: Vec<serde_json::Value> = vec![];

    if n.filters.filter_by_assignee {
        conditions.push(serde_json::json!({
            "property": n.properties.assignee.as_deref().unwrap_or("Assignee"),
            "people": { "contains": n.user_id }
        }));
    }

    for status in &n.filters.exclude_statuses {
        conditions.push(serde_json::json!({
            "property": n.properties.status,
            "status": { "does_not_equal": status }
        }));
    }

    // Sprint filtering only applies when a sprint property is configured —
    // filtering on a property the database doesn't have 400s the whole query.
    if let Some(sprint_prop) = n.properties.sprint.as_deref() {
        let sprint_ids = match sprint_database_id(&n.token, &n.database_id, sprint_prop).await {
            Some(db) => current_sprint_ids(&n.token, &db).await,
            None => vec![],
        };
        if !sprint_ids.is_empty() {
            let mut per_sprint: Vec<serde_json::Value> = sprint_ids
                .iter()
                .map(|id| serde_json::json!({
                    "property": sprint_prop,
                    "relation": { "contains": id }
                }))
                .collect();
            conditions.push(if per_sprint.len() == 1 {
                per_sprint.remove(0)
            } else {
                serde_json::json!({ "or": per_sprint })
            });
        }
    }

    match conditions.len() {
        0 => serde_json::json!({}),
        1 => serde_json::json!({ "filter": conditions.remove(0) }),
        _ => serde_json::json!({ "filter": { "and": conditions } }),
    }
}

/// Every queued task, straight from Notion. No local writes.
pub(crate) async fn fetch_queue() -> anyhow::Result<Vec<ProviderTask>> {
    let cfg = config::require()?;
    let body = queue_filter(&cfg).await;
    let pages = super::api::paginate_post(
        &cfg.notion.token,
        &format!("v1/databases/{}/query", cfg.notion.database_id),
        &body,
        MAX_TASK_PAGES,
    )
    .await?;

    Ok(pages
        .iter()
        .filter_map(|page| match page_to_task(page, &cfg.notion) {
            Ok(task) => Some(task),
            Err(e) => {
                tracing::warn!("skipping page: {e}");
                None
            }
        })
        .collect())
}

/// One page by id.
pub(crate) async fn fetch_page(page_id: &str) -> anyhow::Result<ProviderTask> {
    let cfg = config::require()?;
    let page = super::api::get(&cfg.notion.token, &format!("v1/pages/{page_id}")).await?;
    page_to_task(&page, &cfg.notion)
}

/// Every queued task, mirrored locally in one transaction.
async fn list_tasks_impl(pool: &SqlitePool) -> anyhow::Result<Vec<TaskView>> {
    let tasks = fetch_queue().await?;
    let keep: Vec<String> = tasks.iter().map(|t| t.external_id.clone()).collect();

    let mut tx = pool.begin().await?;
    store::provider_tasks::prune_missing(&mut *tx, "notion", &keep).await?;
    for task in &tasks {
        store::provider_tasks::upsert(&mut *tx, task).await?;
    }
    tx.commit().await?;

    Ok(tasks.into_iter().map(Into::into).collect())
}

/// One task by its short id, via the unique_id property — whose NAME is resolved
/// from the schema (Notion filters take the property's name, not its type).
async fn fetch_by_short_id(cfg: &NotionConfig, short_id: &str) -> anyhow::Result<ProviderTask> {
    let num = parse_short_id_number(short_id)
        .ok_or_else(|| anyhow::anyhow!("Cannot parse short_id: {short_id}"))?;
    let schema = super::schema::load(&cfg.token, &cfg.database_id).await?;
    let id_prop = schema
        .properties
        .iter()
        .find(|p| p.kind == "unique_id")
        .map(|p| p.name.clone())
        .ok_or_else(|| anyhow::anyhow!("the task database has no unique_id property"))?;

    let body = serde_json::json!({
        "filter": { "property": id_prop, "unique_id": { "equals": num } }
    });
    let rows = super::api::paginate_post(
        &cfg.token,
        &format!("v1/databases/{}/query", cfg.database_id),
        &body,
        1,
    )
    .await?;
    let page = rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("Task {short_id} not found in Notion"))?;
    page_to_task(page, cfg)
}

// ─── Status writes ────────────────────────────────────────────────────────────

/// Set the configured status property of a page.
pub async fn set_status(token: &str, page_id: &str, prop_name: &str, status: &str) -> anyhow::Result<()> {
    super::api::patch(
        token,
        &format!("v1/pages/{page_id}"),
        &serde_json::json!({
            "properties": { prop_name: { "status": { "name": status } } }
        }),
    )
    .await?;
    Ok(())
}

/// Move a page to the workspace trash (Notion has no hard delete; it keeps
/// trashed pages for thirty days).
pub async fn trash(token: &str, page_id: &str) -> anyhow::Result<()> {
    super::api::patch(
        token,
        &format!("v1/pages/{page_id}"),
        &serde_json::json!({ "in_trash": true }),
    )
    .await?;
    Ok(())
}

// ─── IPC ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_tasks(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<TaskView>, String> {
    list_tasks_impl(&pool).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_task(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TaskView, String> {
    let cfg = config::require().map_err(|e| e.to_string())?;
    let task = fetch_by_short_id(&cfg.notion, &short_id)
        .await
        .map_err(|e| e.to_string())?;
    store::provider_tasks::upsert(&*pool, &task).await.map_err(|e| e.to_string())?;
    Ok(task.into())
}
