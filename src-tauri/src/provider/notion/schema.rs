//! The task database's own description of itself.
//!
//! Notion already knows every property, its type, its select options and where a
//! relation points. Reading that instead of hardcoding it means the app follows
//! the board: add a property in Notion and it becomes editable here, with no
//! config entry and no release. It also removes guesses that were never really
//! configuration — the Sprint database id, for one, is simply the target of the
//! Sprint relation.

use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};


use super::api;

/// A schema is re-read this often. Long enough to keep property panels snappy,
/// short enough that a new Notion option shows up while you work.
const CACHE_TTL_SECS: i64 = 300;

/// Property types we can render AND write. Anything else is shown read-only
/// rather than hidden — seeing a value you can't edit beats pretending it isn't
/// there.
/// Number properties hours are logged into. There is exactly one plausible name,
/// and guessing wrong would silently write to the wrong column.
pub(super) const HOURS_NAMES: [&str; 3] = ["Hours spent", "Hours", "Time spent"];

/// Not fields the user sets: an id, a timestamp, a computed value.
const META_KINDS: [&str; 6] =
    ["title", "formula", "unique_id", "created_time", "last_edited_time", "rollup"];

const WRITABLE: [&str; 9] = [
    "select",
    "status",
    "number",
    "multi_select",
    "relation",
    "date",
    "checkbox",
    "rich_text",
    "url",
];

pub use crate::provider::types::{PropertySchema, StatusGroup, TaskSchema};

// ─── Cache ────────────────────────────────────────────────────────────────────

type Cache = RwLock<HashMap<String, (i64, TaskSchema)>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Fetch a database's schema, from cache when it is fresh.
pub(crate) async fn load(token: &str, database_id: &str) -> anyhow::Result<TaskSchema> {
    if let Ok(map) = cache().read() {
        if let Some((at, schema)) = map.get(database_id) {
            if now() - at < CACHE_TTL_SECS {
                return Ok(schema.clone());
            }
        }
    }

    let body = api::get(token, &format!("v1/databases/{database_id}")).await?;
    let schema = parse(database_id, &body)?;

    if let Ok(mut map) = cache().write() {
        map.insert(database_id.to_string(), (now(), schema.clone()));
    }
    Ok(schema)
}

fn parse(database_id: &str, body: &serde_json::Value) -> anyhow::Result<TaskSchema> {
    let props = body["properties"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("database {database_id} returned no properties"))?;

    let mut properties: Vec<PropertySchema> = props
        .iter()
        .map(|(name, def)| {
            let kind = def["type"].as_str().unwrap_or("unknown").to_string();
            // select/status/multi_select all nest their options under the type key.
            let options = def[&kind]["options"]
                .as_array()
                .map(|opts| {
                    opts.iter()
                        .filter_map(|o| o["name"].as_str().map(PropertyOption::named))
                        .collect()
                })
                .unwrap_or_default();
            let relation_db = def["relation"]["database_id"].as_str().map(str::to_string);
            PropertySchema {
                editable: WRITABLE.contains(&kind.as_str()),
                meta: META_KINDS.contains(&kind.as_str()),
                name: name.clone(),
                kind,
                options,
                relation_db,
            }
        })
        .collect();
    properties.sort_by(|a, b| a.name.cmp(&b.name));

    let title_property = properties
        .iter()
        .find(|p| p.kind == "title")
        .map(|p| p.name.clone())
        .ok_or_else(|| anyhow::anyhow!("database {database_id} has no title property"))?;

    let status_groups = props
        .values()
        .find(|def| def["type"].as_str() == Some("status"))
        .map(|def| parse_status_groups(&def["status"]))
        .unwrap_or_default();

    let hours_property = properties
        .iter()
        .find(|p| p.kind == "number" && HOURS_NAMES.contains(&p.name.as_str()))
        .map(|p| p.name.clone());

    Ok(TaskSchema {
        database_id: database_id.to_string(),
        title_property,
        properties,
        status_groups,
        hours_property,
    })
}

/// `groups` reference options by id, so resolve them to names.
///
/// The public API returns `groups: [{ name, option_ids }]`; some surfaces key the
/// groups object by an internal name instead (`to_do`, `in_progress`, `complete`).
/// Both are read here, because the caller only ever compares normalized names.
fn parse_status_groups(status: &serde_json::Value) -> Vec<StatusGroup> {
    let name_by_id: HashMap<&str, &str> = status["options"]
        .as_array()
        .map(|opts| {
            opts.iter()
                .filter_map(|o| Some((o["id"].as_str()?, o["name"].as_str()?)))
                .collect()
        })
        .unwrap_or_default();

    if let Some(groups) = status["groups"].as_array() {
        return groups
            .iter()
            .filter_map(|g| {
                Some(StatusGroup {
                    name: g["name"].as_str()?.to_string(),
                    options: g["option_ids"]
                        .as_array()?
                        .iter()
                        .filter_map(|id| name_by_id.get(id.as_str()?).map(|n| n.to_string()))
                        .collect(),
                })
            })
            .collect();
    }
    // Keyed form: { "to_do": [{name}], "in_progress": [...], ... }
    if let Some(groups) = status["groups"].as_object() {
        return groups
            .iter()
            .map(|(name, opts)| StatusGroup {
                name: name.clone(),
                options: opts
                    .as_array()
                    .map(|a| a.iter().filter_map(|o| o["name"].as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
            })
            .collect();
    }
    vec![]
}

// ─── IPC ──────────────────────────────────────────────────────────────────────

/// The configured task database's schema — drives the property panel.

pub use crate::provider::types::PropertyOption;

/// Every row of a relation's target database, so the picker can filter locally.
/// Paginates to a cap — a relation with thousands of rows wants a server-side
/// search instead, and this returns what it got rather than silently truncating
/// to one page.

/// Every row of a relation's target database, as pickable options.
pub(crate) async fn relation_options(
    token: &str,
    database_id: &str,
) -> anyhow::Result<Vec<PropertyOption>> {
    const MAX_PAGES: usize = 5;
    let rows = api::paginate_post(
        token,
        &format!("v1/databases/{database_id}/query"),
        &serde_json::json!({}),
        MAX_PAGES,
    )
    .await?;

    let mut out: Vec<PropertyOption> = rows
        .iter()
        .filter_map(|row| {
            // Find the title by TYPE, not by name: every database names it
            // differently ("Brick / Component" here).
            let title = row["properties"]
                .as_object()
                .and_then(|props| props.values().find(|p| p["type"] == "title"))
                .map(|p| {
                    p["title"]
                        .as_array()
                        .map(|spans| {
                            spans
                                .iter()
                                .filter_map(|s| s["plain_text"].as_str())
                                .collect::<String>()
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            row["id"].as_str().map(|id| PropertyOption { id: id.to_string(), title })
        })
        .collect();

    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from the live `Platform Tasks` database. The point of the test is
    /// that everything the property panel needs — option lists, relation targets,
    /// which fields are writable — comes out of Notion's own answer.
    fn body() -> serde_json::Value {
        serde_json::json!({
            "properties": {
                "Task name": { "type": "title", "title": {} },
                "Hours spent": { "type": "number", "number": { "format": "number" } },
                "Priority": { "type": "select", "select": { "options": [
                    { "name": "Low" }, { "name": "Medium" }, { "name": "High" }
                ]}},
                "Platform Components": { "type": "relation", "relation": {
                    "database_id": "2bddcbda-9c35-80a8-9e81-d8e3928ef295"
                }},
                "Sprint": { "type": "relation", "relation": {
                    "database_id": "775d0850-bf41-43f9-addb-1ef559ad02af"
                }},
                "Time spent (days)": { "type": "formula", "formula": {} },
                "Task ID": { "type": "unique_id", "unique_id": {} }
            }
        })
    }

    #[test]
    fn options_and_relation_targets_come_from_notion() {
        let s = parse("db-1", &body()).expect("schema");
        assert_eq!(s.title_property, "Task name");
        let options: Vec<&str> =
            s.property("Priority").unwrap().options.iter().map(|o| o.title.as_str()).collect();
        assert_eq!(options, ["Low", "Medium", "High"]);
        // This is what replaces the hardcoded sprint database id.
        assert_eq!(
            s.relation_target("Sprint"),
            Some("775d0850-bf41-43f9-addb-1ef559ad02af")
        );
        assert_eq!(
            s.relation_target("Platform Components"),
            Some("2bddcbda-9c35-80a8-9e81-d8e3928ef295")
        );
    }

    #[test]
    fn computed_properties_are_read_only() {
        let s = parse("db-1", &body()).expect("schema");
        assert!(s.property("Hours spent").unwrap().editable);
        assert!(s.property("Priority").unwrap().editable);
        assert!(!s.property("Time spent (days)").unwrap().editable, "a formula is not settable");
        assert!(!s.property("Task ID").unwrap().editable, "unique_id is generated");
        assert!(!s.property("Task name").unwrap().editable, "the title has its own edit path");
    }

    #[test]
    fn a_database_without_a_title_is_rejected() {
        let bad = serde_json::json!({ "properties": { "N": { "type": "number", "number": {} } } });
        assert!(parse("db-1", &bad).is_err());
    }
}
