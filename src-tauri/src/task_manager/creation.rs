//! Filing a new task in Notion.
//!
//! Two callers share this: `task.create` files a task and stops there, and the
//! explorer conversion files one and then adopts the session onto it. The page
//! creation is identical, so it lives here once — conversion is "create + adopt",
//! not a second creation path.

use sqlx::SqlitePool;

use crate::core::db::models::NotionTask;
use crate::core::db::store;

/// The fields both callers put in their confirmation payload. Property names come
/// from config; the token is injected at execution time, never persisted.
pub(super) struct NewTask<'a> {
    pub token: &'a str,
    pub database_id: &'a str,
    pub title: &'a str,
    pub body_markdown: &'a str,
    pub status_prop: &'a str,
    pub status_value: &'a str,
    pub assignee_prop: Option<&'a str>,
    pub user_id: &'a str,
    pub sprint_prop: Option<&'a str>,
    pub project_prop: Option<&'a str>,
    pub project_id: Option<&'a str>,
}

impl<'a> NewTask<'a> {
    pub(super) fn from_payload(payload: &'a serde_json::Value) -> anyhow::Result<Self> {
        let field = |k: &str| -> Option<&'a str> { payload[k].as_str() };
        Ok(Self {
            token: field("token").ok_or_else(|| anyhow::anyhow!("missing token"))?,
            database_id: field("database_id")
                .ok_or_else(|| anyhow::anyhow!("missing database_id"))?,
            title: field("title").unwrap_or("Untitled task"),
            body_markdown: field("body_markdown").unwrap_or(""),
            status_prop: field("status_prop").unwrap_or("Status"),
            status_value: field("status_value").unwrap_or(""),
            assignee_prop: field("assignee_prop"),
            user_id: field("user_id").unwrap_or(""),
            sprint_prop: field("sprint_prop"),
            project_prop: field("project_prop"),
            project_id: field("project_id"),
        })
    }
}

/// Create the page. Returns `(notion_page_id, short_id)` — the short id is read
/// back from Notion's generated unique_id.
pub(super) async fn create_page(req: &NewTask<'_>) -> anyhow::Result<(String, String)> {
    // The sprint database is the Sprint relation's target (see schema.rs).
    let sprint_ids = match req.sprint_prop {
        Some(prop) => match super::notion::sprint_database_id(req.token, req.database_id, prop).await
        {
            Some(db) => super::notion::current_sprint_ids(req.token, &db).await,
            None => vec![],
        },
        None => vec![],
    };
    let children = super::notion::markdown_to_blocks(req.body_markdown);
    super::notion::create_task_page(
        req.token,
        req.database_id,
        req.title,
        req.status_prop,
        req.status_value,
        req.assignee_prop,
        req.user_id,
        req.sprint_prop,
        &sprint_ids,
        req.project_prop,
        req.project_id,
        children,
    )
    .await
}

/// Confirmation-bridge path for `task.create`: file the task and nothing else.
///
/// Deliberately does NOT open a session or provision worktrees — filing a task you
/// intend to pick up later shouldn't clone repositories.
pub async fn create_task_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let req = NewTask::from_payload(&payload)?;
    let (notion_page_id, short_id) = create_page(&req).await?;

    let task = NotionTask {
        page_id: notion_page_id.clone(),
        short_id: short_id.clone(),
        title: req.title.to_string(),
        status: req.status_value.to_string(),
        priority: None,
        synced_at: chrono::Utc::now().timestamp(),
    };
    store::notion_tasks::upsert(pool, &task).await?;

    Ok(serde_json::json!({
        "short_id": short_id,
        "notion_page_id": notion_page_id,
        "title": req.title,
        "status": req.status_value,
        "priority": null,
        "last_synced_at": task.synced_at,
        "message": format!("Filed {short_id}"),
    }))
}

/// Payload shared by both creation entry points. The token is NOT included —
/// `execute_op` injects it so it never lands in a persisted confirmation row.
pub(crate) fn new_task_payload(
    cfg: &crate::core::config::NotionConfig,
    title: &str,
    body_markdown: &str,
) -> serde_json::Value {
    serde_json::json!({
        "database_id": cfg.database_id,
        "title": title,
        "body_markdown": body_markdown,
        "status_prop": cfg.properties.status,
        "status_value": cfg.status_map.ready,
        "assignee_prop": cfg.properties.assignee,
        "user_id": cfg.user_id,
        "sprint_prop": cfg.properties.sprint,
        "project_prop": cfg.properties.project,
        "project_id": cfg.default_project_id,
    })
}

/// Apply the properties the composer set, after the page exists.
///
/// Notion's create-page call takes a fixed set (status, assignee, sprint, project);
/// anything else is a PATCH. The page is already filed by the time we get here, so a
/// property that won't take must NOT fail the whole call — it is reported instead,
/// and the user fixes it on the overview.
async fn apply_extra_properties(
    page_id: &str,
    properties: &std::collections::HashMap<String, serde_json::Value>,
    cfg: &crate::core::config::Config,
    pool: &SqlitePool,
) -> Vec<String> {
    let mut warnings = vec![];
    for (name, value) in properties {
        if let Err(e) = super::properties::set_property(
            &cfg.notion.token,
            &cfg.notion.database_id,
            page_id,
            name,
            value,
            &cfg.notion,
            pool,
        )
        .await
        {
            warnings.push(format!("{name}: {e}"));
        }
    }
    warnings
}

/// File a task from the UI composer. Direct, not gated: you typed it and pressed
/// the button. It is NOT opened or provisioned — it lands in the queue.
#[tauri::command]
pub async fn create_task(
    title: String,
    body_markdown: String,
    properties: Option<std::collections::HashMap<String, serde_json::Value>>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    if title.trim().is_empty() {
        return Err("a task needs a title".into());
    }
    let cfg = crate::core::config::require().map_err(|e| e.to_string())?;
    let mut payload = new_task_payload(&cfg.notion, title.trim(), &body_markdown);
    payload["token"] = serde_json::json!(cfg.notion.token);
    let mut created = create_task_impl(payload, &pool).await.map_err(|e| e.to_string())?;

    if let Some(props) = properties.filter(|p| !p.is_empty()) {
        let page_id = created["notion_page_id"].as_str().unwrap_or_default().to_string();
        let warnings = apply_extra_properties(&page_id, &props, &cfg, &pool).await;
        if !warnings.is_empty() {
            created["warnings"] = serde_json::json!(warnings);
        }
    }
    Ok(created)
}

/// The configured task template as markdown, for the composer to start from.
/// Empty when no template is configured — a blank body is a fine default.
#[tauri::command]
pub async fn get_task_template_markdown() -> Result<String, String> {
    let cfg = crate::core::config::require().map_err(|e| e.to_string())?;
    let Some(page_id) = cfg.notion.task_template_page_id.filter(|s| !s.is_empty()) else {
        return Ok(String::new());
    };
    super::notion::get_task_template_markdown(&page_id, &cfg.notion.token)
        .await
        .map_err(|e| e.to_string())
}
