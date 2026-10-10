use futures_util::StreamExt;
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::{file, token_context};
use crate::{Client, LogQuery, Start};

#[tokio::test]
async fn a_followed_log_asks_for_timestamps_from_where_it_starts_and_streams_its_lines() {
    let server = MockServer::start().await;
    let body = "2026-10-09T14:02:09.1Z ready\n2026-10-09T14:02:10.2Z job 1 done\n";
    Mock::given(path("/api/v1/namespaces/paxone/pods/api-0/log"))
        .and(query_param("container", "worker"))
        .and(query_param("timestamps", "true"))
        .and(query_param("follow", "true"))
        .and(query_param("sinceTime", "2026-10-09T14:02:09Z"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let config = file(dir.path(), "config", &token_context("kind", &server.uri()));
    let client = Client::connect(&[config], "kind").await.expect("a client");
    let start = Start::Time("2026-10-09T14:02:09Z".into());
    let query = LogQuery {
        namespace: "paxone",
        pod: "api-0",
        container: "worker",
        previous: false,
        follow: true,
        start: &start,
    };
    let lines: Vec<String> = client
        .logs(query)
        .await
        .expect("a stream")
        .map(|line| line.expect("a line"))
        .collect()
        .await;
    assert_eq!(
        lines,
        [
            "2026-10-09T14:02:09.1Z ready",
            "2026-10-09T14:02:10.2Z job 1 done"
        ]
    );
}

#[tokio::test]
async fn a_previous_log_the_container_never_had_is_an_api_error() {
    let server = MockServer::start().await;
    let refused = serde_json::json!({ "kind": "Status", "apiVersion": "v1", "status": "Failure",
        "message": "previous terminated container \"worker\" not found", "code": 400 });
    Mock::given(path("/api/v1/namespaces/paxone/pods/api-0/log"))
        .and(query_param("previous", "true"))
        .respond_with(ResponseTemplate::new(400).set_body_json(refused))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let config = file(dir.path(), "config", &token_context("kind", &server.uri()));
    let client = Client::connect(&[config], "kind").await.expect("a client");
    let query = LogQuery {
        namespace: "paxone",
        pod: "api-0",
        container: "worker",
        previous: true,
        follow: false,
        start: &Start::Beginning,
    };
    let said = client.logs(query).await.err().map(|e| e.to_string());
    assert!(said.is_some_and(|one| one.contains("not found")));
}
