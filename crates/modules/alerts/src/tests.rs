use groove_types::{AlertFilter, AlertsConfig, Timestamp};

use crate::parsed;

fn listed(title: &str, priority: &str, team: &str, at: &str) -> serde_json::Value {
    serde_json::json!({
        "id": format!("id-{title}"),
        "title": title,
        "description": "Backup daily-full ended PartiallyFailed.",
        "deduplication_key": format!("key-{title}"),
        "source_url": "https://thanos.example.com/graph",
        "status": "firing",
        "created_at": at,
        "attributes": [
            { "attribute": { "name": "Priority" }, "value": { "literal": "01K", "label": priority } },
            { "attribute": { "name": "Cluster" }, "value": { "literal": "prod-eu", "label": "prod-eu" } },
            { "attribute": { "name": "Namespace" }, "value": { "literal": "velero" } },
            { "attribute": { "name": "Team" }, "value": { "literal": team, "label": team } },
            { "attribute": { "name": "Severity" } },
        ],
    })
}

fn printed(alerts: &[serde_json::Value]) -> String {
    serde_json::Value::Array(alerts.to_vec()).to_string()
}

#[test]
fn an_alert_reads_its_attributes_by_the_names_the_config_gives() {
    let text = printed(&[listed(
        "VeleroBackupFailed",
        "Critical",
        "Platform",
        "2026-11-17T10:05:44.375Z",
    )]);
    let alerts = parsed(&text, &AlertsConfig::default()).unwrap();
    let one = &alerts[0];
    assert_eq!(one.dedup_key, "key-VeleroBackupFailed");
    assert_eq!(one.priority.as_deref(), Some("Critical"));
    assert_eq!(one.cluster.as_deref(), Some("prod-eu"));
    assert_eq!(
        one.namespace.as_deref(),
        Some("velero"),
        "a literal with no label"
    );
    assert_eq!(
        one.created_at,
        Timestamp::parse("2026-11-17T10:05:44Z").unwrap()
    );
}

#[test]
fn the_most_urgent_come_first_then_the_newest() {
    let text = printed(&[
        listed("old-warning", "Warning", "Platform", "2026-11-17T08:00:00Z"),
        listed("new-warning", "Warning", "Platform", "2026-11-17T09:00:00Z"),
        listed("critical", "Critical", "Platform", "2026-11-17T07:00:00Z"),
    ]);
    let titles: Vec<String> = parsed(&text, &AlertsConfig::default())
        .unwrap()
        .into_iter()
        .map(|one| one.title)
        .collect();
    assert_eq!(titles, ["critical", "new-warning", "old-warning"]);
}

#[test]
fn a_filter_keeps_only_the_alerts_whose_attribute_it_names() {
    let text = printed(&[
        listed("ours", "Warning", "Platform", "2026-11-17T08:00:00Z"),
        listed("theirs", "Warning", "Cargo", "2026-11-17T08:00:00Z"),
    ]);
    let config = AlertsConfig {
        only: Some(AlertFilter {
            attribute: "Team".into(),
            values: vec!["Platform".into()],
        }),
        ..AlertsConfig::default()
    };
    let alerts = parsed(&text, &config).unwrap();
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].title, "ours");
}

#[test]
fn what_is_not_a_list_of_alerts_is_an_error() {
    assert!(parsed("not json", &AlertsConfig::default()).is_err());
}
