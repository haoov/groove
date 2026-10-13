//! Notion's block lists come in pages; a body reads every one of them.

use super::*;

const CHILDREN: &str = "/v1/blocks/1f2e3d4c/children";

fn paragraph(text: &str) -> serde_json::Value {
    serde_json::json!({ "type": "paragraph", "paragraph": { "rich_text": [{ "plain_text": text }] } })
}

#[tokio::test]
async fn a_body_past_one_page_of_blocks_reads_whole() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/v1/pages/1f2e3d4c"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path(CHILDREN))
        .and(wiremock::matchers::query_param("start_cursor", "c2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [paragraph("second")], "has_more": false, "next_cursor": null
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path(CHILDREN))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [paragraph("first")], "has_more": true, "next_cursor": "c2"
        })))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let notion = Notion::at(&host, config()).expect("a client");
    let read = notion.fetch(&key()).await.expect("the page answers");
    assert_eq!(read.body, "first\n\nsecond");
}
