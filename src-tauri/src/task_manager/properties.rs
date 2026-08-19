//! Reading and writing a task's Notion properties, driven by the schema.
//!
//! Nothing here knows about Priority or Platform Components specifically: the
//! property's *type* (from schema.rs) decides how a value is read and how the
//! patch is built. Adding a property in Notion makes it editable here with no
//! code change.
//!
//! Values cross the IPC boundary in ONE canonical shape per type — the same shape
//! for reads and writes, so the panel can round-trip what it was given:
//!   select / status / url / date  → string | null
//!   number                        → number | null
//!   checkbox                      → bool
//!   multi_select                  → string[]  (option names)
//!   relation                      → string[]  (page ids)
//!   rich_text                     → string

use serde::Serialize;
use sqlx::SqlitePool;

use super::notion::{notion_get, notion_patch};

/// One property as the panel sees it: what it is, its current value, and a
/// human rendering for the types we can display but not edit.
#[derive(Debug, Clone, Serialize)]
pub struct PropertyValue {
    pub name: String,
    pub kind: String,
    /// Canonical value (see the module docs). `null` when unset.
    pub value: serde_json::Value,
    /// Read-only rendering, used for formulas, rollups, people, timestamps.
    pub display: String,
}

/// Pull one property out of a Notion page into the canonical shape.
fn read_value(kind: &str, prop: &serde_json::Value) -> (serde_json::Value, String) {
    let plain = |arr: &serde_json::Value| -> String {
        arr.as_array()
            .map(|spans| {
                spans
                    .iter()
                    .filter_map(|s| s["plain_text"].as_str())
                    .collect::<String>()
            })
            .unwrap_or_default()
    };

    match kind {
        "select" | "status" => {
            let name = prop[kind]["name"].as_str();
            (
                name.map(|s| serde_json::json!(s)).unwrap_or(serde_json::Value::Null),
                name.unwrap_or("").to_string(),
            )
        }
        "number" => {
            let n = prop["number"].as_f64();
            (
                n.map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null),
                n.map(trim_number).unwrap_or_default(),
            )
        }
        "checkbox" => {
            let b = prop["checkbox"].as_bool().unwrap_or(false);
            (serde_json::json!(b), if b { "yes".into() } else { "no".into() })
        }
        "multi_select" => {
            let names: Vec<String> = prop["multi_select"]
                .as_array()
                .map(|a| a.iter().filter_map(|o| o["name"].as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            (serde_json::json!(names), names.join(", "))
        }
        "relation" => {
            let ids: Vec<String> = prop["relation"]
                .as_array()
                .map(|a| a.iter().filter_map(|o| o["id"].as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let n = ids.len();
            (serde_json::json!(ids), if n == 0 { String::new() } else { format!("{n} linked") })
        }
        "date" => {
            let start = prop["date"]["start"].as_str();
            (
                start.map(|s| serde_json::json!(s)).unwrap_or(serde_json::Value::Null),
                start.unwrap_or("").to_string(),
            )
        }
        "rich_text" => {
            let text = plain(&prop["rich_text"]);
            (serde_json::json!(text), text)
        }
        "url" | "email" | "phone_number" => {
            let v = prop[kind].as_str();
            (
                v.map(|s| serde_json::json!(s)).unwrap_or(serde_json::Value::Null),
                v.unwrap_or("").to_string(),
            )
        }
        "title" => {
            let text = plain(&prop["title"]);
            (serde_json::json!(text), text)
        }
        // Read-only types: no editable value, just something to show.
        "formula" => {
            let f = &prop["formula"];
            let shown = f["string"]
                .as_str()
                .map(str::to_string)
                .or_else(|| f["number"].as_f64().map(trim_number))
                .or_else(|| f["boolean"].as_bool().map(|b| b.to_string()))
                .or_else(|| f["date"]["start"].as_str().map(str::to_string))
                .unwrap_or_default();
            (serde_json::Value::Null, shown)
        }
        "unique_id" => {
            let prefix = prop["unique_id"]["prefix"].as_str().unwrap_or("");
            let num = prop["unique_id"]["number"].as_i64().unwrap_or_default();
            (
                serde_json::Value::Null,
                if prefix.is_empty() { num.to_string() } else { format!("{prefix}-{num}") },
            )
        }
        "people" => {
            let names: Vec<String> = prop["people"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|p| p["name"].as_str().or(p["id"].as_str()).map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            (serde_json::Value::Null, names.join(", "))
        }
        "created_time" | "last_edited_time" => (
            serde_json::Value::Null,
            prop[kind].as_str().unwrap_or("").to_string(),
        ),
        _ => (serde_json::Value::Null, String::new()),
    }
}

/// `3.0` → "3", `3.25` → "3.25". Hours read better without trailing zeros.
fn trim_number(v: f64) -> String {
    if (v - v.trunc()).abs() < f64::EPSILON {
        format!("{}", v.trunc() as i64)
    } else {
        format!("{v}")
    }
}

/// Build the Notion patch body for one property. `null` clears it.
pub(super) fn property_patch(
    kind: &str,
    value: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let null = value.is_null();
    let body = match kind {
        "select" | "status" => {
            let inner = if null {
                serde_json::Value::Null
            } else {
                serde_json::json!({ "name": as_str(value)? })
            };
            serde_json::json!({ kind: inner })
        }
        "number" => serde_json::json!({
            "number": if null { serde_json::Value::Null } else {
                serde_json::json!(value.as_f64().ok_or_else(|| anyhow::anyhow!("number expected"))?)
            }
        }),
        "checkbox" => serde_json::json!({ "checkbox": value.as_bool().unwrap_or(false) }),
        "multi_select" => {
            let names = value.as_array().ok_or_else(|| anyhow::anyhow!("array expected"))?;
            let opts: Vec<serde_json::Value> = names
                .iter()
                .filter_map(|n| n.as_str())
                .map(|n| serde_json::json!({ "name": n }))
                .collect();
            serde_json::json!({ "multi_select": opts })
        }
        "relation" => {
            let ids = value.as_array().ok_or_else(|| anyhow::anyhow!("array expected"))?;
            let rel: Vec<serde_json::Value> = ids
                .iter()
                .filter_map(|n| n.as_str())
                .map(|id| serde_json::json!({ "id": id }))
                .collect();
            serde_json::json!({ "relation": rel })
        }
        "date" => serde_json::json!({
            "date": if null { serde_json::Value::Null } else {
                serde_json::json!({ "start": as_str(value)? })
            }
        }),
        "rich_text" => serde_json::json!({
            "rich_text": if null || as_str(value)?.is_empty() {
                serde_json::json!([])
            } else {
                serde_json::json!([{ "type": "text", "text": { "content": as_str(value)? } }])
            }
        }),
        "url" => serde_json::json!({
            "url": if null || as_str(value)?.is_empty() { serde_json::Value::Null } else { value.clone() }
        }),
        other => {
            return Err(anyhow::anyhow!(
                "{other} properties can't be edited from here — change it in Notion"
            ))
        }
    };
    Ok(body)
}

fn as_str(v: &serde_json::Value) -> anyhow::Result<&str> {
    v.as_str().ok_or_else(|| anyhow::anyhow!("string expected, got {v}"))
}

// ─── Reads ────────────────────────────────────────────────────────────────────

/// Every property of a task page, in schema order, with current values.
#[tauri::command]
pub async fn get_task_properties(notion_page_id: String) -> Result<Vec<PropertyValue>, String> {
    let cfg = crate::core::config::require().map_err(|e| e.to_string())?;
    let page = notion_get(&cfg.notion.token, &format!("v1/pages/{notion_page_id}"))
        .await
        .map_err(|e| e.to_string())?;
    let schema = super::schema::load(&cfg.notion.token, &cfg.notion.database_id)
        .await
        .map_err(|e| e.to_string())?;

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

// ─── Writes ───────────────────────────────────────────────────────────────────

/// Patch one property and re-sync the local task row.
///
/// The tasks table mirrors status and priority (Home and the queue read them), so
/// a write that doesn't refresh it would leave the UI showing the old value until
/// the next full sync.
pub(crate) async fn set_property(
    token: &str,
    database_id: &str,
    notion_page_id: &str,
    property: &str,
    value: &serde_json::Value,
    cfg: &crate::core::config::NotionConfig,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let schema = super::schema::load(token, database_id).await?;
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
    let updated = notion_patch(token, &format!("v1/pages/{notion_page_id}"), &body).await?;

    if let Ok(task) = super::notion::page_to_task(&updated, cfg) {
        let _ = crate::core::db::store::notion_tasks::upsert(pool, &task).await;
    }

    let (_, display) = read_value(&prop.kind, &updated["properties"][property]);
    Ok(display)
}

/// UI path: you clicked it, so it happens. Agent writes go through the
/// confirmation bridge instead (op `notion.property`).
#[tauri::command]
pub async fn update_task_property(
    notion_page_id: String,
    property: String,
    value: serde_json::Value,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    let cfg = crate::core::config::require().map_err(|e| e.to_string())?;
    set_property(
        &cfg.notion.token,
        &cfg.notion.database_id,
        &notion_page_id,
        &property,
        &value,
        &cfg.notion,
        &pool,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Confirmation-bridge path for `notion.property` (agent-initiated).
pub async fn update_property_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let field = |k: &str| payload[k].as_str().unwrap_or_default().to_string();
    let cfg = crate::core::config::require()?;
    let display = set_property(
        &field("token"),
        &cfg.notion.database_id,
        &field("notion_page_id"),
        &field("property"),
        &payload["value"],
        &cfg.notion,
        pool,
    )
    .await?;
    Ok(serde_json::json!({ "property": field("property"), "value": display }))
}
