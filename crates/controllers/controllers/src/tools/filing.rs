//! Where the agent files a new task, and what it sets on it there, from the config.

use groove_types::{Config, ProviderId};
use serde_json::{Value, json};

/// The source a new task goes to, and what it takes: the database and the properties a
/// Notion page is filed with, or the host an issue is opened on.
pub(super) fn file_at(config: Option<&Config>, which: Option<ProviderId>) -> Value {
    let Some(config) = config else {
        return Value::Null;
    };
    let chosen = which.or_else(|| only(config));
    match chosen {
        Some(ProviderId::Notion) => config.notion.as_ref().map_or(Value::Null, |notion| {
            json!({
                "provider": "notion",
                "database_id": notion.database_id,
                "status": {
                    "property": notion.properties.status,
                    "value": notion.status_map.ready.first(),
                },
                "assignee": {
                    "property": notion.assignee(),
                    "user_id": notion.user_id,
                },
            })
        }),
        Some(ProviderId::Github) => config.github.as_ref().map_or(
            Value::Null,
            |github| json!({ "provider": "github", "host": github.host }),
        ),
        None => Value::Null,
    }
}

/// The one source set up, when there is only one.
fn only(config: &Config) -> Option<ProviderId> {
    match (config.notion.is_some(), config.github.is_some()) {
        (true, false) => Some(ProviderId::Notion),
        (false, true) => Some(ProviderId::Github),
        _ => None,
    }
}
