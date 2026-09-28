use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::Notion;

#[tokio::test]
async fn a_database_the_token_reads_becomes_a_source_with_every_name_a_gap() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/databases/DB"))
        .and(header("authorization", "Bearer secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "object": "database" })))
        .mount(&server)
        .await;
    let found = Notion::connect_at(&server.uri(), " secret ", " DB ", "ME")
        .await
        .unwrap();
    assert_eq!(
        (found.token.as_str(), found.database_id.as_str()),
        ("secret", "DB")
    );
    assert_eq!(found.user_id, "ME");
    assert!(found.properties.status.is_empty() && found.properties.logged.is_none());
    assert_eq!(
        found.unmapped(),
        ["assignee", "sprint"],
        "named in the mapping"
    );
}

#[tokio::test]
async fn a_database_the_token_cannot_read_is_refused() {
    let server = MockServer::start().await;
    let refused = json!({ "object": "error", "message": "Could not find database" });
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_json(refused))
        .mount(&server)
        .await;
    let read = Notion::connect_at(&server.uri(), "secret", "DB", "ME").await;
    assert!(read.is_err());
}
