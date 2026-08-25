//! Reading a Notion page's properties: the task extraction and the canonical
//! per-type value shapes the property panel round-trips.
//!
//! Values cross the IPC boundary in ONE canonical shape per type — the same shape
//! for reads and writes:
//!   select / status / url / date  → string | null
//!   number                        → number | null
//!   checkbox                      → bool
//!   multi_select                  → string[]  (option names)
//!   relation                      → string[]  (page ids)
//!   rich_text                     → string


use crate::core::config::NotionConfig;
use crate::core::db::models::ProviderTask;

/// The page's title text, found by TYPE: every database names its title property
/// differently, and the type is unambiguous.
fn extract_title(props: &serde_json::Value) -> Option<String> {
    let obj = props.as_object()?;
    obj.values()
        .find(|v| v["type"].as_str() == Some("title"))
        .map(|v| {
            v["title"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|t| t["plain_text"].as_str()).collect())
                .unwrap_or_default()
        })
        .filter(|t: &String| !t.is_empty())
}

fn extract_status(props: &serde_json::Value, prop_name: &str) -> Option<String> {
    let prop = props.get(prop_name)?;
    prop["status"]["name"]
        .as_str()
        .or_else(|| prop["select"]["name"].as_str())
        .map(str::to_string)
}

fn extract_select(props: &serde_json::Value, prop_name: &str) -> Option<String> {
    props.get(prop_name)?["select"]["name"].as_str().map(str::to_string)
}

pub(super) fn extract_unique_id(props: &serde_json::Value) -> Option<String> {
    let obj = props.as_object()?;
    obj.values().find_map(|val| {
        if val["type"].as_str() != Some("unique_id") {
            return None;
        }
        let num = val["unique_id"]["number"].as_u64()?;
        let prefix = val["unique_id"]["prefix"].as_str().unwrap_or("");
        Some(if prefix.is_empty() { format!("{num}") } else { format!("{prefix}-{num}") })
    })
}

pub fn page_to_task(page: &serde_json::Value, cfg: &NotionConfig) -> anyhow::Result<ProviderTask> {
    let page_id = page["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("page missing id"))?
        .to_string();

    let props = &page["properties"];

    let short_id = extract_unique_id(props)
        .ok_or_else(|| anyhow::anyhow!("page {page_id} has no unique_id"))?;

    let title = extract_title(props).unwrap_or_else(|| format!("(untitled) {short_id}"));

    let status = extract_status(props, &cfg.properties.status)
        .unwrap_or_else(|| cfg.status_map.in_progress.clone());

    let priority = cfg
        .properties
        .priority
        .as_deref()
        .and_then(|k| extract_select(props, k));

    Ok(ProviderTask {
        url: Some(format!("https://www.notion.so/{}", page_id.replace('-', ""))),
        external_id: page_id,
        short_id,
        title,
        status,
        priority,
        synced_at: chrono::Utc::now().timestamp(),
        provider: "notion".to_string(),
        board: None,
        branch_tag: None,
    })
}

// ─── Canonical property values ────────────────────────────────────────────────

pub use crate::provider::types::PropertyValue;

/// Pull one property out of a Notion page into the canonical shape.
pub(super) fn read_value(kind: &str, prop: &serde_json::Value) -> (serde_json::Value, String) {
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
