//! One issue as a task: the board it sits on supplies the properties the config names.

use groove_types::{Day, GithubConfig, Task, TaskDates, TaskKey, Timestamp};

/// The issue as Groove reads it, or nothing when it sits on no board.
pub(super) fn task(issue: &serde_json::Value, host: &str, config: &GithubConfig) -> Option<Task> {
    let item = issue["projectItems"]["nodes"].as_array()?.first()?;
    let owner = text(&issue["repository"]["owner"]["login"]);
    let repo = text(&issue["repository"]["name"]);
    let number = issue["number"].as_u64()?;
    let names = &config.properties;
    let status = field(item, Some(&names.status)).unwrap_or_default();
    let priority = field(item, names.priority.as_deref());
    Some(Task {
        external_id: TaskKey::Github {
            host: host.to_string(),
            owner: owner.clone(),
            repo: repo.clone(),
            number,
        }
        .external_id(),
        short_id: short_id(&owner, &repo, number),
        title: text(&issue["title"]),
        intent: config.status_map.intent(&status),
        status,
        priority: priority.and_then(|value| config.priority_map.level(&value)),
        dates: TaskDates {
            start: day(field(item, names.start.as_deref())),
            due: day(field(item, names.due.as_deref())),
            duration_days: None,
        },
        estimate: field(item, names.estimate.as_deref()).and_then(|v| v.parse().ok()),
        synced_at: Timestamp::now(),
        provider: groove_types::ProviderId::Github,
        url: Some(text(&issue["url"])),
        board: Some(text(&item["project"]["title"])),
        branch_tag: Some(number.to_string()),
    })
}

/// `gh-<owner>-<repo>-<number>`, the name a branch and a worktree carry.
fn short_id(owner: &str, repo: &str, number: u64) -> String {
    format!("gh-{owner}-{repo}-{number}")
}

/// What the board holds for the field of this name, as text.
fn field(item: &serde_json::Value, name: Option<&str>) -> Option<String> {
    let name = name?;
    item["fieldValues"]["nodes"]
        .as_array()?
        .iter()
        .find(|value| value["field"]["name"].as_str() == Some(name))
        .and_then(shown)
}

/// A field value of any kind GitHub answers with, as the text it reads as.
fn shown(value: &serde_json::Value) -> Option<String> {
    match value["__typename"].as_str()? {
        "ProjectV2ItemFieldSingleSelectValue" => Some(text(&value["name"])),
        "ProjectV2ItemFieldTextValue" => Some(text(&value["text"])),
        "ProjectV2ItemFieldDateValue" => Some(text(&value["date"])),
        "ProjectV2ItemFieldNumberValue" => value["number"].as_f64().map(|n| n.to_string()),
        _ => None,
    }
}

fn day(value: Option<String>) -> Option<Day> {
    Day::parse(&value?).ok()
}

fn text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}
