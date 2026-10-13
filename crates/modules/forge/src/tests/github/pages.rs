//! A PR's threads past the first page are read too.

use super::*;
use wiremock::matchers::body_string_contains;

#[tokio::test]
async fn threads_past_the_first_page_are_read() {
    let mut first = pr();
    first["reviewThreads"]["pageInfo"] =
        serde_json::json!({ "hasNextPage": true, "endCursor": "T1" });
    let mut later = first["reviewThreads"]["nodes"][0].clone();
    later["id"] = "THREAD_2".into();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_string_contains("\"after\":\"T1\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "node": { "reviewThreads": {
                "pageInfo": { "hasNextPage": false, "endCursor": null },
                "nodes": [later]
            }}}
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(by_branch(vec![first])))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let github = Github::with_token(&host, Token::Fixed("t".into())).expect("a client");
    let read = github.find_mr(&repo(), "b").await.unwrap().expect("its pr");
    let ids: Vec<&str> = read.threads.iter().map(|one| one.id.as_str()).collect();
    assert_eq!(ids, ["THREAD_1", "THREAD_2", "COMMENT_1"]);
}
