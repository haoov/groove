//! Filing a new task in Notion.


use crate::core::config::NotionConfig;

use super::page::extract_unique_id;

/// What creating a page needs, read off the config. The token is not here: it is
/// read at execution time so it never rides in a persisted payload.
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
    pub fn from_config(cfg: &'a NotionConfig, title: &'a str, body_markdown: &'a str) -> Self {
        Self {
            database_id: &cfg.database_id,
            title,
            body_markdown,
            status_prop: &cfg.properties.status,
            status_value: &cfg.status_map.ready,
            assignee_prop: cfg.properties.assignee.as_deref(),
            user_id: &cfg.user_id,
            sprint_prop: cfg.properties.sprint.as_deref(),
            project_prop: cfg.properties.project.as_deref(),
            project_id: cfg.default_project_id.as_deref(),
        }
    }
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
