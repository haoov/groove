//! Reading and writing a task's properties, driven by the schema.
//!
//! Nothing here knows about Priority or Platform Components specifically: the
//! property's *type* (from schema.rs) decides how a value is read and how the
//! patch is built. Adding a property in Notion makes it editable here with no
//! code change. Value shapes are canonical — see page.rs.

use crate::core::config::NotionConfig;

use super::page::{property_patch, read_value, PropertyValue};

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
