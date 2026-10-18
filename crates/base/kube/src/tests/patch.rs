use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::{file, token_context};
use crate::{Client, Error, Patch};

const API: &str = "/apis/apps/v1/namespaces/paxone/deployments/api";

async fn client(server: &MockServer) -> (tempfile::TempDir, Client) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "config", &token_context("kind", &server.uri()));
    let client = Client::connect(&[path], "kind").await.expect("a client");
    (dir, client)
}

fn scaled() -> serde_json::Value {
    json!({ "metadata": { "name": "api", "resourceVersion": "8",
        "managedFields": [{ "manager": "groove" }] }, "spec": { "replicas": 3 } })
}

#[tokio::test]
async fn a_dry_run_sends_a_merge_patch_on_the_version_seen_and_reads_the_result_bare() {
    let server = MockServer::start().await;
    let sent = json!({ "spec": { "replicas": 3 }, "metadata": { "resourceVersion": "7" } });
    Mock::given(method("PATCH"))
        .and(path(API))
        .and(query_param("dryRun", "All"))
        .and(query_param("fieldManager", "groove"))
        .and(header("content-type", "application/merge-patch+json"))
        .and(body_json(sent))
        .respond_with(ResponseTemplate::new(200).set_body_json(scaled()))
        .expect(1)
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let body = json!({ "spec": { "replicas": 3 } });
    let patch = Patch {
        path: API,
        body: &body,
        version: Some("7"),
        dry_run: true,
    };
    let object = client.patch_object(patch).await.expect("a patched object");
    assert_eq!(object["spec"]["replicas"], 3);
    assert!(object["metadata"].get("managedFields").is_none());
}

#[tokio::test]
async fn a_write_names_no_dry_run_and_no_version_when_none_is_given() {
    let server = MockServer::start().await;
    let sent = json!({ "spec": { "replicas": 3 } });
    Mock::given(method("PATCH"))
        .and(path(API))
        .and(query_param_is_missing("dryRun"))
        .and(body_json(sent.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(scaled()))
        .expect(1)
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let patch = Patch {
        path: API,
        body: &sent,
        version: None,
        dry_run: false,
    };
    client.patch_object(patch).await.expect("a patched object");
}

#[tokio::test]
async fn a_409_is_a_conflict_on_the_moved_object() {
    let server = MockServer::start().await;
    let status = json!({ "kind": "Status", "apiVersion": "v1", "status": "Failure",
        "message": "the object has been modified", "reason": "Conflict", "code": 409 });
    Mock::given(method("PATCH"))
        .and(path(API))
        .respond_with(ResponseTemplate::new(409).set_body_json(status))
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let body = json!({ "spec": { "replicas": 3 } });
    let patch = Patch {
        path: API,
        body: &body,
        version: Some("7"),
        dry_run: false,
    };
    let refused = client.patch_object(patch).await.expect_err("a conflict");
    assert!(matches!(refused, Error::Conflict { context } if context == "kind"));
}
