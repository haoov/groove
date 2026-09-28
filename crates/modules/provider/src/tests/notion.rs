use groove_types::{
    FilterConfig, NotionConfig, Priority, PriorityMap, PropertyNames, StatusIntent, StatusMap,
    TaskKey,
};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::Notion;

fn config() -> NotionConfig {
    NotionConfig {
        token: "secret".into(),
        database_id: "DB".into(),
        user_id: "USER".into(),
        assignee: Some("Owner".into()),
        sprint: Some("Sprint".into()),
        sprint_status: None,
        properties: PropertyNames {
            status: "State".into(),
            priority: Some("Priority".into()),
            start: Some("Start".into()),
            due: Some("Due".into()),
            estimate: Some("Estimate".into()),
            estimate_unit: groove_types::EstimateUnit::Hours,
            logged: Some("Spent".into()),
        },
        status_map: StatusMap {
            ready: vec!["Todo".into()],
            in_progress: vec!["Doing".into()],
            done: vec!["Done".into()],
        },
        priority_map: PriorityMap {
            high: vec!["P1".into()],
            medium: vec!["P2".into()],
            low: vec!["P3".into()],
        },
        filters: FilterConfig {
            exclude_statuses: vec!["Done".into()],
        },
        task_template_page_id: None,
        default_project_id: None,
    }
}

fn page() -> serde_json::Value {
    serde_json::json!({
        "object": "page",
        "id": "1f2e3d4c",
        "url": "https://notion.so/1f2e3d4c",
        "properties": {
            "ID": { "type": "unique_id", "unique_id": { "prefix": "TASKS2", "number": 4244 } },
            "Name": { "type": "title", "title": [{ "plain_text": "Migrate the database" }] },
            "State": { "type": "status", "status": { "name": "Doing" } },
            "Priority": { "type": "select", "select": { "name": "P1" } },
            "Start": { "type": "date", "date": { "start": "2026-09-14" } },
            "Due": { "type": "date", "date": { "start": "2026-09-30T18:00:00.000+02:00" } },
            "Estimate": { "type": "number", "number": 6.5 },
            "Spent": { "type": "number", "number": 1.5 }
        }
    })
}

fn blocks() -> serde_json::Value {
    serde_json::json!({
        "object": "list",
        "results": [
            { "type": "paragraph", "paragraph": { "rich_text": [{ "plain_text": "The runbook." }] } },
            { "type": "bulleted_list_item",
              "bulleted_list_item": { "rich_text": [{ "plain_text": "stop the writes" }] } },
            { "type": "to_do",
              "to_do": { "checked": true, "rich_text": [{ "plain_text": "take a backup" }] } }
        ]
    })
}

/// A server that answers every call with `reply`, and the source on it.
async fn source(reply: serde_json::Value) -> (MockServer, Notion) {
    source_with(reply, config()).await
}

async fn source_with(reply: serde_json::Value, config: NotionConfig) -> (MockServer, Notion) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply.clone()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply.clone()))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let notion = Notion::at(&host, config).expect("a client");
    (server, notion)
}

fn key() -> TaskKey {
    TaskKey::Notion {
        page_id: "1f2e3d4c".into(),
    }
}

#[tokio::test]
async fn a_page_reads_as_a_task_through_the_mapping() {
    let reply = serde_json::json!({ "object": "list", "results": [page()] });
    let (_server, notion) = source(reply).await;
    let tasks = notion.list().await.expect("the query answers");
    let task = tasks.first().expect("one task");
    assert_eq!(task.short_id, "TASKS2-4244");
    assert_eq!(task.title, "Migrate the database");
    assert_eq!(task.status, "Doing");
    assert_eq!(task.intent, Some(StatusIntent::InProgress));
    assert_eq!(task.priority, Some(Priority::High));
    assert_eq!(task.dates.start.map(|day| day.day), Some(14));
    assert_eq!(task.dates.due.map(|day| day.day), Some(30), "past the time");
    assert_eq!(task.estimate, Some(6.5));
    assert_eq!(task.logged, Some(1.5));
    assert_eq!(task.external_id.as_str(), "1f2e3d4c");
}

#[tokio::test]
async fn the_query_asks_for_yours_and_leaves_the_excluded_statuses_out() {
    let reply = serde_json::json!({ "object": "list", "results": [] });
    let (server, notion) = source(reply).await;
    notion.list().await.expect("the query answers");
    let sent: serde_json::Value = server
        .received_requests()
        .await
        .expect("the calls")
        .iter()
        .find(|call| call.url.path() == "/v1/databases/DB/query")
        .expect("the tasks are asked")
        .body_json()
        .expect("json");
    let and = sent["filter"]["and"].as_array().expect("both terms");
    assert_eq!(and[0]["property"], "Owner");
    assert_eq!(and[0]["people"]["contains"], "USER");
    assert_eq!(and[1]["property"], "State");
    assert_eq!(and[1]["status"]["does_not_equal"], "Done");
}

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
async fn a_page_without_an_id_of_its_own_is_not_a_task() {
    let mut bare = page();
    bare["properties"]["ID"] = serde_json::json!({ "type": "number", "number": 1 });
    let reply = serde_json::json!({ "object": "list", "results": [bare] });
    let (_server, notion) = source(reply).await;
    assert!(notion.list().await.expect("the query answers").is_empty());
}

#[tokio::test]
async fn a_read_brings_the_page_and_its_blocks_as_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/v1/pages/1f2e3d4c"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/v1/blocks/1f2e3d4c/children"))
        .respond_with(ResponseTemplate::new(200).set_body_json(blocks()))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let notion = Notion::at(&host, config()).expect("a client");
    let read = notion.fetch(&key()).await.expect("the page answers");
    assert_eq!(read.task.short_id, "TASKS2-4244");
    assert_eq!(
        read.body,
        "The runbook.\n- stop the writes\n- [x] take a backup"
    );
}

#[tokio::test]
async fn a_status_is_written_as_the_label_the_map_names() {
    let (server, notion) = source(page()).await;
    let label = notion
        .set_status(&key(), StatusIntent::Done)
        .await
        .expect("the write lands");
    assert_eq!(label, "Done");
    let sent: Vec<serde_json::Value> = server
        .received_requests()
        .await
        .expect("the calls")
        .iter()
        .filter(|call| call.method == wiremock::http::Method::PATCH)
        .map(|call| call.body_json().expect("json"))
        .collect();
    let write = sent.last().expect("the patch");
    assert_eq!(write["properties"]["State"]["status"]["name"], "Done");
}

#[tokio::test]
async fn logging_hours_adds_them_to_what_the_page_already_holds() {
    let (server, notion) = source(page()).await;
    let whole = notion
        .log_hours(&key(), 0.5)
        .await
        .expect("the write lands");
    assert_eq!(whole, 2.0);
    let sent: Vec<serde_json::Value> = server
        .received_requests()
        .await
        .expect("the calls")
        .iter()
        .filter(|call| call.method == wiremock::http::Method::PATCH)
        .map(|call| call.body_json().expect("json"))
        .collect();
    let write = sent.last().expect("the patch");
    assert_eq!(write["properties"]["Spent"]["number"], 2.0);
}

#[tokio::test]
async fn what_notion_refuses_comes_back_as_the_reason_it_gave() {
    let reply = serde_json::json!({ "object": "error", "message": "Unauthorized" });
    let (_server, notion) = source(reply).await;
    let refused = notion.list().await.expect_err("the query is refused");
    assert!(refused.to_string().contains("Unauthorized"), "{refused}");
}

#[tokio::test]
async fn a_template_page_is_read_as_the_markdown_a_task_starts_from() {
    let (_server, notion) = source(blocks()).await;
    assert_eq!(notion.template().await.expect("a read"), None);

    let mut config = config();
    config.task_template_page_id = Some("TEMPLATE_1".into());
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(blocks()))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let notion = Notion::at(&host, config).expect("a client");
    let held = notion
        .template()
        .await
        .expect("a read")
        .expect("a template");
    assert!(held.starts_with("The runbook."), "{held}");
    assert!(held.contains("- [x] take a backup"), "{held}");
}

#[tokio::test]
async fn an_estimate_the_source_counts_in_days_reads_as_hours() {
    let mut config = config();
    config.properties.estimate_unit = groove_types::EstimateUnit::Days;
    let reply = serde_json::json!({ "object": "list", "results": [page()] });
    let (_server, notion) = source_with(reply, config).await;
    let tasks = notion.list().await.expect("the query answers");
    assert_eq!(
        tasks[0].estimate,
        Some(52.0),
        "six days and a half, at eight hours a day"
    );
}
