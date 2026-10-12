//! Notion's sprint: the query keeps the running one, or fails when it cannot read it.

use super::*;

/// The same config, asked to keep only the sprint that is running.
fn with_sprint() -> NotionConfig {
    NotionConfig {
        sprint: Some("Sprint".into()),
        ..config()
    }
}

#[tokio::test]
async fn the_query_keeps_the_tasks_of_the_sprint_that_is_running() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/v1/databases/DB"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "database",
            "properties": { "Sprint": { "relation": { "database_id": "SPRINTS" } } }
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/v1/databases/SPRINTS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "database",
            "properties": { "Sprint status": { "type": "status" } }
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path("/v1/databases/SPRINTS/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "results": [{ "id": "SPRINT_9" }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path("/v1/databases/DB/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "results": [page()]
        })))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let notion = Notion::at(&host, with_sprint()).expect("a client");
    assert_eq!(notion.list().await.expect("the query answers").len(), 1);

    let calls = server.received_requests().await.expect("the calls");
    let sprints: serde_json::Value = calls
        .iter()
        .find(|call| call.url.path() == "/v1/databases/SPRINTS/query")
        .expect("the sprint database is asked")
        .body_json()
        .expect("json");
    assert_eq!(sprints["filter"]["property"], "Sprint status");
    assert_eq!(sprints["filter"]["status"]["equals"], "Current");
    let tasks: serde_json::Value = calls
        .iter()
        .find(|call| call.url.path() == "/v1/databases/DB/query")
        .expect("the tasks are asked")
        .body_json()
        .expect("json");
    let and = tasks["filter"]["and"].as_array().expect("three terms");
    assert_eq!(and[2]["property"], "Sprint");
    assert_eq!(and[2]["relation"]["contains"], "SPRINT_9");
}

#[tokio::test]
async fn a_sprint_property_the_database_lacks_leaves_the_query_alone() {
    let reply = serde_json::json!({ "object": "list", "results": [page()] });
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "database",
            "properties": {}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let notion = Notion::at(&host, with_sprint()).expect("a client");
    assert_eq!(notion.list().await.expect("the query answers").len(), 1);
    let tasks: serde_json::Value = server
        .received_requests()
        .await
        .expect("the calls")
        .iter()
        .find(|call| call.url.path() == "/v1/databases/DB/query")
        .expect("the tasks are asked")
        .body_json()
        .expect("json");
    assert!(
        tasks["filter"]["and"]
            .as_array()
            .is_some_and(|and| and.len() == 2),
        "no sprint term at all: {tasks}"
    );
}

#[tokio::test]
async fn nothing_is_listed_until_the_assignee_and_the_sprint_are_named() {
    let unnamed = NotionConfig {
        assignee: None,
        sprint: Some(" ".into()),
        ..config()
    };
    let notion = Notion::at("http://127.0.0.1:9", unnamed).expect("a client");
    let refused = notion.list().await.expect_err("refused").to_string();
    assert!(refused.contains("assignee and sprint"), "{refused}");
}

#[tokio::test]
async fn a_sprint_that_cannot_be_read_fails_the_list_instead_of_widening_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "results": [page()]
        })))
        .mount(&server)
        .await;
    let notion = Notion::at(&format!("http://{}", server.address()), config()).expect("a client");
    assert!(notion.list().await.is_err(), "no task of another sprint");
}
