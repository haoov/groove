//! A Notion page's blocks, and the blocks nested under them, as Markdown.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::notion::body::{Node, text};

fn run(text: &str, marks: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "plain_text": text, "annotations": marks })
}

fn block(kind: &str, runs: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "type": kind, kind: { "rich_text": runs } })
}

#[test]
fn a_page_s_blocks_read_as_markdown_a_blank_line_apart() {
    let run = |text: &str, marks: serde_json::Value| serde_json::json!({ "plain_text": text, "annotations": marks });
    let block = |kind: &str, runs: Vec<serde_json::Value>| serde_json::json!({ "type": kind, kind: { "rich_text": runs } });
    let nodes: Vec<Node> = [
        block("heading_2", vec![run("Description", serde_json::json!({}))]),
        block(
            "paragraph",
            vec![
                run("Close ", serde_json::json!({})),
                run("the gates ", serde_json::json!({ "bold": true })),
                run("now", serde_json::json!({ "code": true })),
            ],
        ),
        block(
            "heading_2",
            vec![run("Why is it needed", serde_json::json!({}))],
        ),
        block(
            "bulleted_list_item",
            vec![run("one", serde_json::json!({}))],
        ),
        block(
            "bulleted_list_item",
            vec![run("two", serde_json::json!({}))],
        ),
        serde_json::json!({ "type": "divider", "divider": {} }),
    ]
    .into_iter()
    .map(|block| Node {
        block,
        children: Vec::new(),
    })
    .collect();
    assert_eq!(
        text(&nodes),
        "## Description\n\nClose **the gates** `now`\n\n## Why is it needed\n\n- one\n- two\n\n---"
    );
}

/// A block of this kind and text, with blocks under it on the server.
fn parent(id: &str, kind: &str, said: &str) -> serde_json::Value {
    let mut one = block(kind, vec![run(said, serde_json::json!({}))]);
    one["id"] = id.into();
    one["has_children"] = true.into();
    one
}

async fn answering(server: &MockServer, id: &str, blocks: Vec<serde_json::Value>) {
    Mock::given(method("GET"))
        .and(path(format!("/v1/blocks/{id}/children")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "results": blocks })),
        )
        .mount(server)
        .await;
}

#[tokio::test]
async fn the_blocks_under_a_list_item_or_a_toggle_are_read_and_nested() {
    let server = MockServer::start().await;
    let plain = serde_json::json!({});
    answering(
        &server,
        "TEMPLATE",
        vec![
            parent("B1", "bulleted_list_item", "outer"),
            parent("T1", "toggle", "Details"),
        ],
    )
    .await;
    answering(
        &server,
        "B1",
        vec![parent("B2", "numbered_list_item", "inner")],
    )
    .await;
    answering(
        &server,
        "B2",
        vec![parent("B3", "bulleted_list_item", "deepest")],
    )
    .await;
    answering(
        &server,
        "T1",
        vec![block("paragraph", vec![run("hidden", plain)])],
    )
    .await;
    let mut config = super::notion::config();
    config.task_template_page_id = Some("TEMPLATE".into());
    let notion =
        crate::Notion::at(&format!("http://{}", server.address()), config).expect("a client");
    let said = notion
        .template()
        .await
        .expect("a read")
        .expect("a template");
    assert_eq!(
        said,
        "- outer\n  1. inner\n     - deepest\n\nDetails\n\nhidden"
    );
    let calls = server.received_requests().await.expect("the calls");
    assert!(
        !calls
            .iter()
            .any(|one| one.url.path() == "/v1/blocks/B3/children"),
        "two levels"
    );
}
