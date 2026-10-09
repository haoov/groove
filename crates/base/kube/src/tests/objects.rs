use wiremock::matchers::{header, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::{file, token_context};
use crate::{Client, Kind, Narrowed, Seen};

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

fn pod(name: &str) -> serde_json::Value {
    serde_json::json!({ "metadata": { "name": name, "uid": name,
        "managedFields": [{ "manager": "kubelet" }] }, "spec": {} })
}

async fn client(server: &MockServer) -> (tempfile::TempDir, Client) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "config", &token_context("kind", &server.uri()));
    let client = Client::connect(&[path], "kind").await.expect("a client");
    (dir, client)
}

#[tokio::test]
async fn whole_objects_list_a_page_at_a_time_without_their_managed_fields() {
    let server = MockServer::start().await;
    let first = serde_json::json!({ "metadata": { "continue": "next", "resourceVersion": "8" },
        "items": [pod("api-0")] });
    let last =
        serde_json::json!({ "metadata": { "resourceVersion": "9" }, "items": [pod("api-1")] });
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("fieldSelector", "metadata.name=api-0"))
        .and(query_param("continue", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(last))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("fieldSelector", "metadata.name=api-0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(first))
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let kind = pods();
    let narrowed = Narrowed {
        kind: &kind,
        namespace: Some("paxone"),
        fields: Some("metadata.name=api-0"),
        labels: None,
    };
    let listed = client.list_objects(narrowed).await.expect("a list");
    assert_eq!(listed.version, "9");
    let names: Vec<_> = listed
        .objects
        .iter()
        .map(|one| one["metadata"]["name"].clone())
        .collect();
    assert_eq!(names, ["api-0", "api-1"]);
    assert!(listed.objects[0]["metadata"].get("managedFields").is_none());
}

#[tokio::test]
async fn a_watch_of_whole_objects_reads_puts_deletions_and_bookmarks() {
    let server = MockServer::start().await;
    let lines = [
        serde_json::json!({ "type": "MODIFIED", "object": pod("api-0") }),
        serde_json::json!({ "type": "DELETED", "object": pod("api-1") }),
        serde_json::json!({ "type": "BOOKMARK", "object": { "metadata": { "resourceVersion": "12" } } }),
    ];
    let body: String = lines.iter().map(|one| format!("{one}\n")).collect();
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("watch", "1"))
        .and(query_param("resourceVersion", "9"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let kind = pods();
    let narrowed = Narrowed {
        kind: &kind,
        namespace: Some("paxone"),
        fields: None,
        labels: None,
    };
    let stream = client.watch_objects(narrowed, "9").await.expect("a watch");
    let seen: Vec<Seen> = futures_util::StreamExt::collect::<Vec<_>>(stream)
        .await
        .into_iter()
        .map(|one| one.expect("an event"))
        .collect();
    assert!(matches!(&seen[0], Seen::Put(one) if one["metadata"].get("managedFields").is_none()));
    assert!(matches!(&seen[1], Seen::Gone(one) if one["metadata"]["name"] == "api-1"));
    assert_eq!(seen[2], Seen::Mark("12".into()));
}

#[tokio::test]
async fn a_metadata_list_asks_for_the_metadata_alone() {
    let server = MockServer::start().await;
    let list = serde_json::json!({ "items": [{ "metadata": { "name": "sh.helm.release.v1.paxone.v3",
        "labels": { "owner": "helm", "name": "paxone", "version": "3" } } }] });
    Mock::given(path("/api/v1/namespaces/paxone/secrets"))
        .and(query_param("labelSelector", "owner=helm,name=paxone"))
        .and(header("accept", crate::objects::AS_METADATA))
        .respond_with(ResponseTemplate::new(200).set_body_json(list))
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let secrets = Kind {
        kind: "Secret".into(),
        plural: "secrets".into(),
        ..pods()
    };
    let narrowed = Narrowed {
        kind: &secrets,
        namespace: Some("paxone"),
        fields: None,
        labels: Some("owner=helm,name=paxone"),
    };
    let found = client.list_metadata(narrowed).await.expect("a list");
    assert_eq!(found[0]["metadata"]["labels"]["version"], "3");
}
