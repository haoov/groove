//! Filing a new task in Notion.
//!
//! Two callers share this: `task.create` files a task and stops there, and the
//! explorer conversion files one and then adopts the session onto it. The page
//! creation is identical, so it lives here once — conversion is "create + adopt",
//! not a second creation path.

use sqlx::SqlitePool;

use crate::core::config::{self, NotionConfig};
use crate::core::db::models::ProviderTask;
use crate::core::db::store;

use super::page::extract_unique_id;

/// The fields both callers put in their confirmation payload. Property names come
/// from config; the token never rides in a payload — it is read from config at
/// execution time.
pub struct NewTask<'a> {
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
    pub fn from_payload(payload: &'a serde_json::Value) -> anyhow::Result<Self> {
        let field = |k: &str| -> Option<&'a str> { payload[k].as_str() };
        Ok(Self {
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

/// Payload shared by both creation entry points.
pub fn new_task_payload(cfg: &NotionConfig, title: &str, body_markdown: &str) -> serde_json::Value {
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

/// Create the page. Returns `(notion_page_id, short_id)` — the short id is read
/// back from Notion's generated unique_id.
pub async fn create_page(token: &str, req: &NewTask<'_>) -> anyhow::Result<(String, String)> {
    // The sprint database is the Sprint relation's target (see schema.rs).
    let sprint_ids = match req.sprint_prop {
        Some(prop) => match super::schema::load(token, req.database_id)
            .await
            .ok()
            .and_then(|s| s.relation_target(prop).map(str::to_string))
        {
            Some(db) => super::tasks::current_sprint_ids(token, &db).await,
            None => vec![],
        },
        None => vec![],
    };

    let title_prop = super::schema::load(token, req.database_id).await?.title_property;

    let mut properties = serde_json::Map::new();
    properties.insert(title_prop, serde_json::json!({ "title": [{ "text": { "content": req.title } }] }));
    if !req.status_value.is_empty() {
        properties.insert(
            req.status_prop.to_string(),
            serde_json::json!({ "status": { "name": req.status_value } }),
        );
    }
    if let Some(ap) = req.assignee_prop {
        if !req.user_id.is_empty() {
            properties.insert(ap.to_string(), serde_json::json!({ "people": [{ "id": req.user_id }] }));
        }
    }
    if let Some(sp) = req.sprint_prop {
        if !sprint_ids.is_empty() {
            let rel: Vec<_> = sprint_ids.iter().map(|id| serde_json::json!({ "id": id })).collect();
            properties.insert(sp.to_string(), serde_json::json!({ "relation": rel }));
        }
    }
    if let (Some(pp), Some(pid)) = (req.project_prop, req.project_id) {
        properties.insert(pp.to_string(), serde_json::json!({ "relation": [{ "id": pid }] }));
    }

    // Notion caps `children` at 100 blocks on page create — send the first 100
    // with the create and append the rest in follow-up batches.
    let mut children = super::markdown::markdown_to_blocks(req.body_markdown);
    let rest = if children.len() > 100 { children.split_off(100) } else { vec![] };

    let body = serde_json::json!({
        "parent": { "database_id": req.database_id },
        "properties": properties,
        "children": children,
    });

    let page = super::api::post(token, "v1/pages", &body).await?;
    let notion_page_id = page["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("created page missing id"))?
        .to_string();
    let short_id = extract_unique_id(&page["properties"]).ok_or_else(|| {
        anyhow::anyhow!("created page has no unique_id — is that property configured on the DB?")
    })?;

    for chunk in rest.chunks(100) {
        super::api::patch(
            token,
            &format!("v1/blocks/{notion_page_id}/children"),
            &serde_json::json!({ "children": chunk }),
        )
        .await?;
    }

    Ok((notion_page_id, short_id))
}

/// Confirmation-bridge path for `task.create`: file the task and nothing else.
///
/// Deliberately does NOT open a session or provision worktrees — filing a task you
/// intend to pick up later shouldn't clone repositories.
pub async fn create_task_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let cfg = config::notion()?;
    let req = NewTask::from_payload(&payload)?;
    let (notion_page_id, short_id) = create_page(&cfg.token, &req).await?;

    let task = ProviderTask {
        external_id: notion_page_id.clone(),
        provider: "notion".to_string(),
        url: Some(format!("https://www.notion.so/{}", notion_page_id.replace('-', ""))),
        board: None,
        branch_tag: None,
        short_id: short_id.clone(),
        title: req.title.to_string(),
        status: req.status_value.to_string(),
        priority: None,
        synced_at: chrono::Utc::now().timestamp(),
    };
    store::provider_tasks::upsert(pool, &task).await?;

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
    let cfg = config::notion().map_err(|e| e.to_string())?;
    let payload = new_task_payload(&cfg, title.trim(), &body_markdown);
    let mut created = create_task_impl(payload, &pool).await.map_err(|e| e.to_string())?;

    // Notion's create call takes a fixed property set; anything else the composer
    // set is a follow-up patch. The page is already filed by then, so a property
    // that won't take must NOT fail the whole call — it is reported instead.
    if let Some(props) = properties.filter(|p| !p.is_empty()) {
        let page_id = created["notion_page_id"].as_str().unwrap_or_default().to_string();
        let warnings = super::properties::set_properties(&page_id, &props, &cfg, &pool).await;
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
    let cfg = config::notion().map_err(|e| e.to_string())?;
    let Some(page_id) = cfg.task_template_page_id.filter(|s| !s.is_empty()) else {
        return Ok(String::new());
    };
    super::body::template_markdown(&page_id, &cfg.token)
        .await
        .map_err(|e| e.to_string())
}
