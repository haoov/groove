//! One page as a task: the properties the config names, as Groove reads them.

use groove_types::{Day, NotionConfig, ProviderId, Task, TaskDates, TaskKey, Timestamp};

/// The query the config describes: yours, not excluded, in a running sprint.
pub(super) fn filter(
    config: &NotionConfig,
    (assignee, sprint): (&str, &str),
    sprints: &[String],
) -> serde_json::Value {
    let mut and = vec![serde_json::json!({
        "property": assignee,
        "people": { "contains": config.user_id }
    })];
    for status in &config.filters.exclude_statuses {
        and.push(serde_json::json!({
            "property": config.properties.status,
            "status": { "does_not_equal": status }
        }));
    }
    and.extend(in_sprint(sprint, sprints));
    match and.len() {
        1 => serde_json::json!({ "filter": and.remove(0) }),
        _ => serde_json::json!({ "filter": { "and": and } }),
    }
}

/// The term that keeps the tasks of the sprints that are running.
fn in_sprint(name: &str, sprints: &[String]) -> Option<serde_json::Value> {
    let mut terms: Vec<serde_json::Value> = sprints
        .iter()
        .map(|id| serde_json::json!({ "property": name, "relation": { "contains": id } }))
        .collect();
    match terms.len() {
        0 => None,
        1 => Some(terms.remove(0)),
        _ => Some(serde_json::json!({ "or": terms })),
    }
}

/// The page as Groove reads it, or nothing when it carries no id of its own.
pub(super) fn task(page: &serde_json::Value, config: &NotionConfig) -> Option<Task> {
    let page_id = page["id"].as_str()?.to_string();
    let short_id = unique(page)?;
    let names = &config.properties;
    let status = select(page, Some(&names.status)).unwrap_or_default();
    Some(Task {
        external_id: TaskKey::Notion {
            page_id: page_id.clone(),
        }
        .external_id(),
        title: title(page).unwrap_or_else(|| short_id.clone()),
        short_id,
        intent: config.status_map.intent(&status),
        status,
        priority: select(page, names.priority.as_deref())
            .and_then(|value| config.priority_map.level(&value)),
        dates: TaskDates {
            start: day(page, names.start.as_deref(), "start"),
            due: day(page, names.due.as_deref(), "end")
                .or_else(|| day(page, names.due.as_deref(), "start")),
            duration_days: None,
        },
        estimate: names
            .estimate
            .as_deref()
            .and_then(|name| number(page, name))
            .map(|value| names.estimate_unit.hours(value)),
        logged: names.logged.as_deref().and_then(|name| number(page, name)),
        synced_at: Timestamp::now(),
        provider: ProviderId::Notion,
        url: page["url"].as_str().map(str::to_string),
        project: None,
        branch_tag: None,
    })
}

/// A page's own id, as the database numbers it.
fn unique(page: &serde_json::Value) -> Option<String> {
    let value = held(page).find_map(|(_, value)| match value["type"].as_str() {
        Some("unique_id") => Some(&value["unique_id"]),
        _ => None,
    })?;
    let number = value["number"].as_i64()?;
    Some(match value["prefix"].as_str() {
        Some(prefix) => format!("{prefix}-{number}"),
        None => number.to_string(),
    })
}

pub(super) fn title(page: &serde_json::Value) -> Option<String> {
    let value = held(page).find_map(|(_, value)| match value["type"].as_str() {
        Some("title") => Some(&value["title"]),
        _ => None,
    })?;
    let text = words(value);
    (!text.is_empty()).then_some(text)
}

/// What a status or select property holds, as text.
pub(super) fn select(page: &serde_json::Value, name: Option<&str>) -> Option<String> {
    let value = property(page, name?)?;
    let held = match value["type"].as_str()? {
        "status" => &value["status"],
        "select" => &value["select"],
        _ => return None,
    };
    held["name"].as_str().map(str::to_string)
}

/// What a number property holds.
pub(super) fn number(page: &serde_json::Value, name: &str) -> Option<f32> {
    property(page, name)?["number"]
        .as_f64()
        .map(|one| one as f32)
}

/// One end of a date property: `start`, or `end` when it holds a range.
fn day(page: &serde_json::Value, name: Option<&str>, end: &str) -> Option<Day> {
    let value = property(page, name?)?;
    let date = value["date"][end].as_str()?;
    Day::parse(date.get(..10).unwrap_or(date)).ok()
}

fn property<'a>(page: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    page["properties"].get(name)
}

fn held(page: &serde_json::Value) -> impl Iterator<Item = (&String, &serde_json::Value)> {
    page["properties"]
        .as_object()
        .map(|held| held.iter())
        .into_iter()
        .flatten()
}

/// Every run of rich text, as one string.
fn words(value: &serde_json::Value) -> String {
    value
        .as_array()
        .map(|runs| {
            runs.iter()
                .filter_map(|run| run["plain_text"].as_str())
                .collect::<String>()
        })
        .unwrap_or_default()
}
