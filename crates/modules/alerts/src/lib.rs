//! The firing alerts incident.io holds, read through the `inc` CLI the user logged in with.

use groove_exec::run::Run;
use groove_types::{Alert, AlertsConfig, Error, ErrorKind, Result, Timestamp};
use serde_json::Value;

#[cfg(test)]
mod tests;

/// Every firing alert the config keeps, the most urgent first, the newest first within a rank.
pub async fn firing(config: &AlertsConfig) -> Result<Vec<Alert>> {
    let run = Run::new("inc").args(["alerts", "list", "--status", "firing"]);
    let printed = run
        .args(["--output", "json", "--limit", "0"])
        .text()
        .await
        .map_err(|e| Error::new(ErrorKind::Provider, e.to_string()))?;
    parsed(&printed, config)
}

/// The alerts in what `inc alerts list` printed.
pub fn parsed(printed: &str, config: &AlertsConfig) -> Result<Vec<Alert>> {
    let listed: Vec<Value> = serde_json::from_str(printed)
        .map_err(|e| Error::new(ErrorKind::Provider, format!("inc printed no alerts: {e}")))?;
    let mut alerts: Vec<Alert> = listed
        .iter()
        .filter(|one| kept(one, config))
        .filter_map(|one| alert(one, config))
        .collect();
    alerts.sort_by_key(|one| (one.rank(), std::cmp::Reverse(one.created_at)));
    Ok(alerts)
}

fn alert(one: &Value, config: &AlertsConfig) -> Option<Alert> {
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
        created_at: Timestamp::parse(one["created_at"].as_str()?).ok()?,
    })
}

/// Whether the config's filter, when it has one, keeps the alert.
fn kept(one: &Value, config: &AlertsConfig) -> bool {
    let Some(only) = &config.only else {
        return true;
    };
    attribute(one, &only.attribute).is_some_and(|value| only.values.contains(&value))
}

/// The label of the attribute of that name, or its literal when it has no label.
fn attribute(one: &Value, name: &str) -> Option<String> {
    let all = one["attributes"].as_array()?;
    let found = all.iter().find(|at| at["attribute"]["name"] == name)?;
    let value = &found["value"];
    let said = value["label"].as_str().or(value["literal"].as_str())?;
    (!said.is_empty()).then(|| said.to_string())
}
