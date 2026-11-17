//! One alert as incident.io lists it, its attributes read by the names the config gives.

use groove_types::{Alert, AlertsConfig, Timestamp};
use serde_json::Value;

pub(crate) fn alert(one: &Value, config: &AlertsConfig) -> Option<Alert> {
    let text = |name: &str| one[name].as_str().map(str::to_string);
    Some(Alert {
        id: text("id")?,
        title: text("title")?,
        description: text("description").unwrap_or_default(),
        dedup_key: text("deduplication_key").or_else(|| text("id"))?,
        source_url: text("source_url").filter(|url| !url.is_empty()),
        priority: attribute(one, &config.priority),
        cluster: attribute(one, &config.cluster),
        namespace: attribute(one, &config.namespace),
        environment: attribute(one, &config.environment),
        created_at: Timestamp::parse(one["created_at"].as_str()?).ok()?,
    })
}

/// The label of the attribute of that name, or its literal when it has no label.
fn attribute(one: &Value, name: &str) -> Option<String> {
    let all = one["attributes"].as_array()?;
    let found = all.iter().find(|at| at["attribute"]["name"] == name)?;
    let value = &found["value"];
    let said = value["label"].as_str().or(value["literal"].as_str())?;
    (!said.is_empty()).then(|| said.to_string())
}
