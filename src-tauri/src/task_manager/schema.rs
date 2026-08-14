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

use serde::Serialize;

use super::notion::notion_get;

/// A schema is re-read this often. Long enough to keep property panels snappy,
/// short enough that a new Notion option shows up while you work.
const CACHE_TTL_SECS: i64 = 300;

/// Property types we can render AND write. Anything else is shown read-only
/// rather than hidden — seeing a value you can't edit beats pretending it isn't
/// there.
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

#[derive(Debug, Clone, Serialize)]
pub struct PropertySchema {
    pub name: String,
    /// Notion's own type string — the frontend renders by this.
    pub kind: String,
    /// Allowed values for select / status / multi_select, in Notion's order.
    pub options: Vec<String>,
    /// Target database for a relation, so its rows can be offered as choices.
    pub relation_db: Option<String>,
    /// False for formulas, rollups and timestamps: displayable, not settable.
    pub editable: bool,
}

/// A status property's option groups, as Notion itself classifies them: To-do,
/// In progress, Complete. This is the ONLY non-guess signal for what an option
/// means — "Fixed with required action" is a completion state and no amount of
/// name matching would say so.
#[derive(Debug, Clone, Serialize)]
pub struct StatusGroup {
    /// Notion's group name ("To-do", "In progress", "Complete"), verbatim.
    pub name: String,
    /// Option names in the group, in Notion's order.
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSchema {
    pub database_id: String,
    /// The title property's name — it differs per database ("Task name" here).
    pub title_property: String,
    pub properties: Vec<PropertySchema>,
    /// Groups of the first `status` property, empty when the database has none.
    pub status_groups: Vec<StatusGroup>,
}

impl TaskSchema {
    pub fn property(&self, name: &str) -> Option<&PropertySchema> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// The database a relation property points at, e.g. the sprint or component
    /// database. `None` when the property is absent or isn't a relation.
    pub fn relation_target(&self, property: &str) -> Option<&str> {
        self.property(property)?.relation_db.as_deref()
    }
}

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
pub(super) async fn load(token: &str, database_id: &str) -> anyhow::Result<TaskSchema> {
    if let Ok(map) = cache().read() {
        if let Some((at, schema)) = map.get(database_id) {
            if now() - at < CACHE_TTL_SECS {
                return Ok(schema.clone());
            }
        }
    }

    let body = notion_get(token, &format!("v1/databases/{database_id}")).await?;
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
                        .filter_map(|o| o["name"].as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let relation_db = def["relation"]["database_id"].as_str().map(str::to_string);
            PropertySchema {
                editable: WRITABLE.contains(&kind.as_str()),
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

    Ok(TaskSchema {
        database_id: database_id.to_string(),
        title_property,
        properties,
        status_groups,
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
#[tauri::command]
pub async fn get_task_schema(
    app: tauri::AppHandle,
    task_state: tauri::State<'_, super::State>,
) -> Result<TaskSchema, String> {
    let cfg = super::ensure_config(&app, &task_state).map_err(|e| e.to_string())?;
    load(&cfg.notion.token, &cfg.notion.database_id)
        .await
        .map_err(|e| e.to_string())
}

/// One choice for a relation property: a page in the target database.
#[derive(Debug, Clone, Serialize)]
pub struct RelationOption {
    pub id: String,
    pub title: String,
}

/// Every row of a relation's target database, so the picker can filter locally.
/// Paginates to a cap — a relation with thousands of rows wants a server-side
/// search instead, and this returns what it got rather than silently truncating
/// to one page.
#[tauri::command]
pub async fn list_relation_options(
    database_id: String,
    app: tauri::AppHandle,
    task_state: tauri::State<'_, super::State>,
) -> Result<Vec<RelationOption>, String> {
    const MAX_PAGES: usize = 5;
    let cfg = super::ensure_config(&app, &task_state).map_err(|e| e.to_string())?;
    let token = &cfg.notion.token;

    let mut out: Vec<RelationOption> = vec![];
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let mut body = serde_json::json!({ "page_size": 100 });
        if let Some(c) = &cursor {
            body["start_cursor"] = serde_json::json!(c);
        }
        let page = super::notion::notion_post(
            token,
            &format!("v1/databases/{database_id}/query"),
            &body,
        )
        .await
        .map_err(|e| e.to_string())?;

        for row in page["results"].as_array().unwrap_or(&vec![]) {
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
            if let Some(id) = row["id"].as_str() {
                out.push(RelationOption { id: id.to_string(), title });
            }
        }

        match page["next_cursor"].as_str() {
            Some(c) if page["has_more"].as_bool().unwrap_or(false) => cursor = Some(c.to_string()),
            _ => break,
        }
    }

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
        assert_eq!(
            s.property("Priority").unwrap().options,
            vec!["Low", "Medium", "High"]
        );
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
