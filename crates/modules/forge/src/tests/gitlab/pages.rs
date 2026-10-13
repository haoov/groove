//! An MR's discussions past the first page are read too.

use super::*;
use wiremock::matchers::body_string_contains;

#[tokio::test]
async fn discussions_past_the_first_page_are_read() {
    let mut first = mr("opened");
    first["discussions"]["pageInfo"] =
        serde_json::json!({ "hasNextPage": true, "endCursor": "D1" });
    let mut later = first["discussions"]["nodes"][0].clone();
    later["id"] = "gid://gitlab/Discussion/2".into();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("\"after\":\"D1\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "project": { "mergeRequest": { "discussions": {
                "pageInfo": { "hasNextPage": false, "endCursor": null },
                "nodes": [later]
            }}}}
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(by_branch(vec![first])))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let gitlab = Gitlab::with_token(&host, Token::Fixed("t".into())).expect("a client");
    let read = gitlab.find_mr(&repo(), "b").await.unwrap().expect("its mr");
    assert_eq!(read.threads.len(), 2, "{:?}", read.threads);
}
