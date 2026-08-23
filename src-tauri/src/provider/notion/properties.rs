//! Reading and writing a task's properties, driven by the schema.
//!
//! Nothing here knows about Priority or Platform Components specifically: the
//! property's *type* (from schema.rs) decides how a value is read and how the
//! patch is built. Adding a property in Notion makes it editable here with no
//! code change. Value shapes are canonical — see page.rs.

use sqlx::SqlitePool;

use crate::core::config::NotionConfig;
use crate::core::db::store;

use super::page::{page_to_task, property_patch, read_value, PropertyValue};

/// Every property of a task page, in schema order, with current values.
pub(crate) async fn read_all(cfg: &NotionConfig, page_id: &str) -> anyhow::Result<Vec<PropertyValue>> {
    let page = super::api::get(&cfg.token, &format!("v1/pages/{page_id}")).await?;
    let schema = super::schema::load(&cfg.token, &cfg.database_id).await?;

    Ok(schema
        .properties
        .iter()
        .map(|p| {
            let prop = &page["properties"][&p.name];
            let (value, display) = read_value(&p.kind, prop);
            PropertyValue { name: p.name.clone(), kind: p.kind.clone(), value, display }
        })
        .collect())
}


/// Patch one property and re-sync the local task row.
///
/// The tasks table mirrors status and priority (Home and the queue read them), so
/// a write that doesn't refresh it would leave the UI showing the old value until
/// the next full sync.
pub(crate) async fn set_property(
    notion_page_id: &str,
    property: &str,
    value: &serde_json::Value,
    cfg: &NotionConfig,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let schema = super::schema::load(&cfg.token, &cfg.database_id).await?;
    let prop = schema
        .property(property)
        .ok_or_else(|| anyhow::anyhow!("{property} is not a property of this database"))?;
    if !prop.editable {
        return Err(anyhow::anyhow!(
            "{property} is a {} — Notion computes it, so it can't be set",
            prop.kind
        ));
    }

    let body = serde_json::json!({
        "properties": { property: property_patch(&prop.kind, value)? }
    });
    let updated = super::api::patch(&cfg.token, &format!("v1/pages/{notion_page_id}"), &body).await?;

    if let Ok(task) = page_to_task(&updated, cfg) {
        let _ = store::provider_tasks::upsert(pool, &task).await;
    }

    let (_, display) = read_value(&prop.kind, &updated["properties"][property]);
    Ok(display)
}

/// Patch one property without touching the local mirror.
pub(crate) async fn patch_property(
    cfg: &NotionConfig,
    page_id: &str,
    property: &str,
    value: &serde_json::Value,
) -> anyhow::Result<String> {
    let schema = super::schema::load(&cfg.token, &cfg.database_id).await?;
    let prop = schema
        .property(property)
        .ok_or_else(|| anyhow::anyhow!("{property} is not a property of this database"))?;
    if !prop.editable {
        anyhow::bail!("{property} is a {} — Notion computes it, so it can't be set", prop.kind);
    }
    let body = serde_json::json!({
        "properties": { property: property_patch(&prop.kind, value)? }
    });
    let updated = super::api::patch(&cfg.token, &format!("v1/pages/{page_id}"), &body).await?;
    let (_, display) = read_value(&prop.kind, &updated["properties"][property]);
    Ok(display)
}

/// Set several properties in ONE patch, falling back to per-property writes only
/// when the batch fails — that isolates the culprit and keeps the good ones.
/// Returns the warnings for the properties that did not take.
pub(super) async fn set_properties(
    notion_page_id: &str,
    properties: &std::collections::HashMap<String, serde_json::Value>,
    cfg: &NotionConfig,
    pool: &SqlitePool,
) -> Vec<String> {
    let batch = batch_patch(properties, cfg).await;
    if let Ok(body) = &batch {
        if let Ok(updated) = super::api::patch(&cfg.token, &format!("v1/pages/{notion_page_id}"), body).await {
            if let Ok(task) = page_to_task(&updated, cfg) {
                let _ = store::provider_tasks::upsert(pool, &task).await;
            }
            return vec![];
        }
    }

    let mut warnings = vec![];
    for (name, value) in properties {
        if let Err(e) = set_property(notion_page_id, name, value, cfg, pool).await {
            warnings.push(format!("{name}: {e}"));
        }
    }
    warnings
}

/// One patch body covering every requested property; errs if any of them fails
/// schema validation (the caller then falls back per-property).
async fn batch_patch(
    properties: &std::collections::HashMap<String, serde_json::Value>,
    cfg: &NotionConfig,
) -> anyhow::Result<serde_json::Value> {
    let schema = super::schema::load(&cfg.token, &cfg.database_id).await?;
    let mut map = serde_json::Map::new();
    for (name, value) in properties {
        let prop = schema
            .property(name)
            .ok_or_else(|| anyhow::anyhow!("{name} is not a property of this database"))?;
        if !prop.editable {
            return Err(anyhow::anyhow!("{name} is not editable"));
        }
        map.insert(name.clone(), property_patch(&prop.kind, value)?);
    }
    Ok(serde_json::json!({ "properties": map }))
}

/// UI path: you clicked it, so it happens. Agent writes go through the
/// confirmation bridge instead (op `task.property`).
#[tauri::command]
pub async fn update_task_property(
    short_id: String,
    property: String,
    value: serde_json::Value,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    write_property(&short_id, &property, &value, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Confirmation-bridge path for `task.property` (agent-initiated).
pub async fn update_property_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let field = |k: &str| payload[k].as_str().unwrap_or_default().to_string();
    let display =
        write_property(&field("task_id"), &field("property"), &payload["value"], pool).await?;
    Ok(serde_json::json!({ "property": field("property"), "value": display }))
}

/// Set one property through the task's own provider, then re-mirror it: Home and
/// the queue read status and priority from the local row.
pub(crate) async fn write_property(
    short_id: &str,
    property: &str,
    value: &serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    let written = provider.set_property(&key, property, value).await?;
    if let Ok(task) = provider.fetch_task(&key).await {
        let _ = store::provider_tasks::upsert(pool, &crate::provider::mirror_row(short_id, &task)).await;
    }
    Ok(written.display)
}
