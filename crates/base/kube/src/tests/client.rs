use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::{file, token_context};
use crate::{Client, Error};

fn version() -> serde_json::Value {
    serde_json::json!({
        "major": "1", "minor": "33", "gitVersion": "v1.33.4",
        "gitCommit": "", "gitTreeState": "clean", "buildDate": "",
        "goVersion": "go1.24", "compiler": "gc", "platform": "linux/amd64"
    })
}

async fn client_on(server: &str) -> (tempfile::TempDir, Client) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "config", &token_context("kind", server));
    let client = Client::connect(&[path], "kind").await.expect("a client");
    (dir, client)
}

#[tokio::test]
async fn a_signed_in_context_answers_its_version_with_the_token_of_the_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/version"))
        .and(header("authorization", "Bearer not-a-real-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(version()))
        .mount(&server)
        .await;
    let (_dir, client) = client_on(&server.uri()).await;
    assert_eq!(client.version().await.expect("a version"), "v1.33.4");
}

#[tokio::test]
async fn a_401_is_a_refused_sign_in() {
    let server = MockServer::start().await;
    let status = serde_json::json!({
        "kind": "Status", "apiVersion": "v1", "status": "Failure",
        "message": "Unauthorized", "reason": "Unauthorized", "code": 401
    });
    Mock::given(path("/version"))
        .respond_with(ResponseTemplate::new(401).set_body_json(status))
        .mount(&server)
        .await;
    let (_dir, client) = client_on(&server.uri()).await;
    let refused = client.version().await;
    assert!(matches!(refused, Err(Error::Refused { .. })), "{refused:?}");
}

#[tokio::test]
async fn a_server_that_does_not_listen_is_unreachable() {
    let (_dir, client) = client_on("http://127.0.0.1:9").await;
    let silent = client.version().await;
    assert!(
        matches!(silent, Err(Error::Unreachable { .. })),
        "{silent:?}"
    );
}

#[tokio::test]
async fn a_context_no_file_names_is_refused_before_any_call() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(
        dir.path(),
        "config",
        &token_context("kind", "http://127.0.0.1:9"),
    );
    let missing = Client::connect(&[path], "prod").await;
    assert!(matches!(missing, Err(Error::NoContext(name)) if name == "prod"));
}
