use groove_token::Token;
use groove_types::{AlertsConfig, Timestamp};
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{Incident, PLATFORM, TEAM};

fn listed(title: &str, priority: &str, at: &str) -> Value {
    json!({
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
            { "attribute": { "name": "Environment Kind" }, "value": { "literal": "01E", "label": "Production" } },
            { "attribute": { "name": "Severity" } },
        ],
    })
}

/// A server that answers the Platform team's firing alerts.
async fn serving(alerts: Vec<Value>) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/alerts"))
        .and(query_param("status[one_of]", "firing"))
        .and(query_param(format!("attributes[{TEAM}][one_of]"), PLATFORM))
        .and(header("authorization", "Bearer secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "alerts": alerts })))
        .mount(&server)
        .await;
    server
}

fn incident(server: &MockServer) -> Incident {
    Incident::at(server.uri(), Token::Fixed("secret".into())).unwrap()
}

#[tokio::test]
async fn incident_io_is_asked_for_the_platform_team_s_firing_alerts_alone() {
    let server = serving(vec![listed(
        "VeleroBackupFailed",
        "Critical",
        "2026-11-17T10:05:44.375Z",
    )])
    .await;
    let alerts = incident(&server)
        .firing(&AlertsConfig::default())
        .await
        .unwrap();
    let one = &alerts[0];
    assert_eq!(one.dedup_key, "key-VeleroBackupFailed");
    assert_eq!(one.priority.as_deref(), Some("Critical"));
    assert_eq!(one.cluster.as_deref(), Some("prod-eu"));
    assert_eq!(
        one.namespace.as_deref(),
        Some("velero"),
        "a literal with no label"
    );
    assert_eq!(one.environment.as_deref(), Some("Production"));
    assert_eq!(
        one.created_at,
        Timestamp::parse("2026-11-17T10:05:44Z").unwrap()
    );
}

#[tokio::test]
async fn the_most_urgent_come_first_then_the_newest() {
    let server = serving(vec![
        listed("old-warning", "Warning", "2026-11-17T08:00:00Z"),
        listed("new-warning", "Warning", "2026-11-17T09:00:00Z"),
        listed("critical", "Critical", "2026-11-17T07:00:00Z"),
    ])
    .await;
    let read = incident(&server)
        .firing(&AlertsConfig::default())
        .await
        .unwrap();
    let titles: Vec<String> = read.into_iter().map(|one| one.title).collect();
    assert_eq!(titles, ["critical", "new-warning", "old-warning"]);
}

#[tokio::test]
async fn a_read_follows_every_page() {
    let server = serving(Vec::new()).await;
    Mock::given(method("GET"))
        .and(path("/v2/alerts"))
        .and(query_param("after", "CURSOR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "alerts": [listed("second-page", "Warning", "2026-11-17T08:00:00Z")],
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/alerts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "alerts": [listed("first-page", "Critical", "2026-11-17T08:00:00Z")],
            "pagination_meta": { "after": "CURSOR" },
        })))
        .with_priority(2)
        .mount(&server)
        .await;
    let read = incident(&server)
        .firing(&AlertsConfig::default())
        .await
        .unwrap();
    let titles: Vec<String> = read.into_iter().map(|one| one.title).collect();
    assert_eq!(titles, ["first-page", "second-page"]);
}
