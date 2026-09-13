use crate::Method;
use crate::client::*;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn headers_bearer_and_body_reach_the_server() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/pages"))
        .and(header("authorization", "Bearer t0k"))
        .and(header("notion-version", "2022-06-28"))
        .and(body_json(serde_json::json!({"a": 1})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "p1"})))
        .mount(&server)
        .await;

    let client = Client::new().unwrap();
    let reply: serde_json::Value = client
        .request(Method::POST, format!("{}/v1/pages", server.uri()))
        .bearer("t0k")
        .header("Notion-Version", "2022-06-28")
        .json(&serde_json::json!({"a": 1}))
        .send_json()
        .await
        .unwrap();
    assert_eq!(reply["id"], "p1");
}

#[tokio::test]
async fn a_failure_status_is_an_error_with_the_message() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(422).set_body_json(serde_json::json!({"message": "nope"})),
        )
        .mount(&server)
        .await;

    let err = Client::new()
        .unwrap()
        .request(Method::GET, format!("{}/x", server.uri()))
        .send_json::<serde_json::Value>()
        .await
        .unwrap_err();
    assert!(
        matches!(err, crate::Error::Status { status: 422, .. }),
        "{err}"
    );
    assert!(err.to_string().ends_with("422: nope"), "{err}");
}
