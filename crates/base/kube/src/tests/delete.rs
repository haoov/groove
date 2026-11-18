use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::{file, token_context};
use crate::{Client, Delete, Error};

const API: &str = "/apis/apps/v1/namespaces/paxone/deployments/api";

async fn client(server: &MockServer) -> (tempfile::TempDir, Client) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "config", &token_context("kind", &server.uri()));
    let client = Client::connect(&[path], "kind").await.expect("a client");
    (dir, client)
}

#[tokio::test]
async fn a_dry_run_delete_says_so_in_its_body_on_the_version_seen() {
    let server = MockServer::start().await;
    let sent = json!({ "kind": "DeleteOptions", "apiVersion": "v1",
        "propagationPolicy": "Background", "preconditions": { "resourceVersion": "7" },
        "dryRun": ["All"] });
    Mock::given(method("DELETE"))
        .and(path(API))
        .and(query_param("dryRun", "All"))
        .and(body_json(sent))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "kind": "Status" })))
        .expect(1)
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let delete = Delete {
        path: API,
        version: Some("7"),
        dry_run: true,
    };
    let checked = client.delete_object(delete).await;
    checked.expect("a delete checked");
}

#[tokio::test]
async fn a_delete_names_no_dry_run_and_a_409_is_a_conflict() {
    let server = MockServer::start().await;
    let status = json!({ "kind": "Status", "apiVersion": "v1", "status": "Failure",
        "message": "the object has been modified", "reason": "Conflict", "code": 409 });
    Mock::given(method("DELETE"))
        .and(path(API))
        .and(query_param_is_missing("dryRun"))
        .respond_with(ResponseTemplate::new(409).set_body_json(status))
        .expect(1)
        .mount(&server)
        .await;
    let (_dir, client) = client(&server).await;
    let delete = Delete {
        path: API,
        version: Some("7"),
        dry_run: false,
    };
    let refused = client.delete_object(delete).await.expect_err("a conflict");
    assert!(matches!(refused, Error::Conflict { context } if context == "kind"));
}
