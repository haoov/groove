use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::{file, token_context};
use crate::{Change, Client, Column, Error, Kind, Query, Row};

fn pods() -> Kind {
    Kind {
        group: String::new(),
        version: "v1".into(),
        kind: "Pod".into(),
        plural: "pods".into(),
        namespaced: true,
        watchable: true,
    }
}

fn row(uid: &str, name: &str, status: &str) -> serde_json::Value {
    serde_json::json!({
        "cells": [name, "1/1", status, 0],
        "object": { "metadata": {
            "uid": uid, "name": name, "namespace": "paxone", "resourceVersion": "7",
            "managedFields": [{ "manager": "kubelet" }]
        } }
    })
}

fn table(rows: Vec<serde_json::Value>, columns: bool) -> serde_json::Value {
    let columns = match columns {
        true => serde_json::json!([
            { "name": "Name", "priority": 0 }, { "name": "Ready", "priority": 0 },
            { "name": "Status", "priority": 0 }, { "name": "IP", "priority": 1 }
        ]),
        false => serde_json::json!([]),
    };
    serde_json::json!({
        "kind": "Table", "apiVersion": "meta.k8s.io/v1",
        "metadata": { "resourceVersion": "42", "continue": "" },
        "columnDefinitions": columns, "rows": rows
    })
}

async fn client(server: &MockServer) -> (tempfile::TempDir, Client) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "config", &token_context("kind", &server.uri()));
    let client = Client::connect(&[path], "kind").await.expect("a client");
    (dir, client)
}

#[tokio::test]
async fn a_page_asks_for_the_table_in_its_namespace_and_reads_cells_and_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("labelSelector", "app=api"))
        .and(query_param("includeObject", "Metadata"))
        .and(query_param("resourceVersion", "0"))
        .and(query_param("resourceVersionMatch", "NotOlderThan"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(table(vec![row("u1", "api-0", "Running")], true)),
        )
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let kind = pods();
    let query = Query {
        kind: &kind,
        namespace: Some("paxone"),
        selector: Some("app=api"),
    };
    let page = client.page(query, None).await.expect("a page");
    let asked = server.received_requests().await.expect("recorded");
    let accept = asked[0]
        .headers
        .get("accept")
        .map(|one| one.to_str().unwrap_or_default().to_string());
    assert_eq!(accept.as_deref(), Some(crate::table::AS_TABLE));
    assert_eq!(
        page.columns[3],
        Column {
            name: "IP".into(),
            priority: 1
        }
    );
    assert_eq!(page.version, "42");
    assert_eq!(page.next, None);
    let one = Row {
        uid: "u1".into(),
        name: "api-0".into(),
        namespace: Some("paxone".into()),
        version: "7".into(),
        cells: vec!["api-0".into(), "1/1".into(), "Running".into(), "0".into()],
    };
    assert_eq!(page.rows, [one]);
}

#[tokio::test]
async fn a_watch_reads_each_event_as_the_change_it_makes_and_a_410_as_expired() {
    let server = MockServer::start().await;
    let lines = [
        serde_json::json!({ "type": "ADDED", "object": table(vec![row("u2", "api-1", "Pending")], true) }),
        serde_json::json!({ "type": "DELETED", "object": table(vec![row("u1", "api-0", "Running")], false) }),
        serde_json::json!({ "type": "BOOKMARK", "object": { "kind": "Table", "apiVersion": "meta.k8s.io/v1", "metadata": { "resourceVersion": "50" } } }),
    ];
    let body: String = lines.iter().map(|one| format!("{one}\n")).collect();
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("watch", "1"))
        .and(query_param("resourceVersion", "42"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let gone = serde_json::json!({ "kind": "Status", "code": 410, "message": "too old", "reason": "Expired" });
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("resourceVersion", "1"))
        .respond_with(ResponseTemplate::new(410).set_body_json(gone))
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let kind = pods();
    let query = Query {
        kind: &kind,
        namespace: Some("paxone"),
        selector: None,
    };
    let stream = client.watch(query, "42").await.expect("a watch");
    let changes: Vec<Change> = futures_util::StreamExt::collect::<Vec<_>>(stream)
        .await
        .into_iter()
        .flat_map(|one| one.expect("an event"))
        .collect();
    assert!(matches!(&changes[0], Change::Columns(columns) if columns.len() == 4));
    assert!(matches!(&changes[1], Change::Put(row) if row.name == "api-1"));
    assert!(matches!(&changes[2], Change::Gone(row) if row.uid == "u1"));
    assert_eq!(changes[3..], [Change::Mark("50".into())]);
    let expired = client.watch(query, "1").await.expect("a watch");
    let first = futures_util::StreamExt::collect::<Vec<_>>(expired).await;
    assert!(
        matches!(first.as_slice(), [Err(Error::Expired { .. })]),
        "{first:?}"
    );
}

#[test]
fn a_cluster_wide_kind_or_no_namespace_lists_across_the_cluster() {
    let nodes = Kind {
        namespaced: false,
        plural: "nodes".into(),
        ..pods()
    };
    assert_eq!(nodes.path(Some("paxone")), "/api/v1/nodes");
    let deployments = Kind {
        group: "apps".into(),
        plural: "deployments".into(),
        ..pods()
    };
    assert_eq!(deployments.path(None), "/apis/apps/v1/deployments");
    assert_eq!(
        deployments.path(Some("a")),
        "/apis/apps/v1/namespaces/a/deployments"
    );
}

#[tokio::test]
async fn discovery_falls_back_to_one_group_at_a_time_and_says_what_lists_and_watches() {
    let server = MockServer::start().await;
    let versions = serde_json::json!({ "kind": "APIVersions", "versions": ["v1"], "serverAddressByClientCIDRs": [] });
    let resources = serde_json::json!({
        "kind": "APIResourceList", "groupVersion": "v1",
        "resources": [
            { "name": "pods", "singularName": "pod", "namespaced": true, "kind": "Pod",
              "verbs": ["get", "list", "watch", "delete"] },
            { "name": "pods/log", "singularName": "", "namespaced": true, "kind": "Pod", "verbs": ["get"] },
            { "name": "bindings", "singularName": "binding", "namespaced": true, "kind": "Binding", "verbs": ["create"] }
        ]
    });
    let groups = serde_json::json!({ "kind": "APIGroupList", "apiVersion": "v1", "groups": [] });
    let plain = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    Mock::given(path("/api"))
        .respond_with(plain(versions))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1"))
        .respond_with(plain(resources))
        .mount(&server)
        .await;
    Mock::given(path("/apis"))
        .respond_with(plain(groups))
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let kinds = client.kinds().await.expect("the kinds");
    let watchable: Vec<(&str, bool)> = kinds
        .iter()
        .map(|one| (one.plural.as_str(), one.watchable))
        .collect();
    assert_eq!(watchable, [("pods", true), ("bindings", false)]);
}
