use groove_types::{GithubConfig, Priority, PriorityMap, PropertyNames, StatusIntent, StatusMap};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{Github, Token};

fn config(host: &str) -> GithubConfig {
    GithubConfig {
        host: host.to_string(),
        token: None,
        properties: PropertyNames {
            status: "Status".into(),
            priority: Some("Priority".into()),
            start: Some("Start".into()),
            due: Some("Due".into()),
            estimate: Some("Estimate".into()),
            estimate_unit: groove_types::EstimateUnit::Hours,
            logged: Some("Spent".into()),
        },
        status_map: StatusMap {
            ready: vec!["Todo".into()],
            in_progress: vec!["In progress".into()],
            done: vec!["Done".into()],
        },
        priority_map: PriorityMap {
            high: vec!["P1".into()],
            medium: vec!["P2".into()],
            low: vec!["P3".into()],
        },
    }
}

fn issue() -> serde_json::Value {
    serde_json::json!({
        "number": 50,
        "title": "Harden Groove",
        "url": "https://github.com/haoov/groove/issues/50",
        "body": "the body",
        "repository": { "name": "groove", "owner": { "login": "haoov" } },
        "projectItems": { "nodes": [{
            "id": "ITEM_1",
            "project": {
                "id": "BOARD_1",
                "title": "Platform",
                "fields": { "nodes": [
                    { "id": "FIELD_SPENT", "name": "Spent" },
                    { "id": "FIELD_STATUS", "name": "Status", "options": [
                        { "id": "OPT_TODO", "name": "Todo" },
                        { "id": "OPT_DOING", "name": "In progress" },
                        { "id": "OPT_DONE", "name": "Done" }
                    ]}
                ]}
            },
            "fieldValues": { "nodes": [
                { "__typename": "ProjectV2ItemFieldSingleSelectValue",
                  "name": "In progress", "field": { "name": "Status" } },
                { "__typename": "ProjectV2ItemFieldSingleSelectValue",
                  "name": "P1", "field": { "name": "Priority" } },
                { "__typename": "ProjectV2ItemFieldDateValue",
                  "date": "2026-09-14", "field": { "name": "Start" } },
                { "__typename": "ProjectV2ItemFieldDateValue",
                  "date": "2026-09-30", "field": { "name": "Due" } },
                { "__typename": "ProjectV2ItemFieldNumberValue",
                  "number": 6.5, "field": { "name": "Estimate" } },
                { "__typename": "ProjectV2ItemFieldNumberValue",
                  "number": 1.5, "field": { "name": "Spent" } }
            ]}
        }]}
    })
}

/// The task every write test names.
fn key() -> groove_types::TaskKey {
    groove_types::TaskKey::Github {
        host: "github.com".into(),
        owner: "haoov".into(),
        repo: "groove".into(),
        number: 50,
    }
}

/// A server that answers every GraphQL call with `reply`, and the source on it.
async fn source(reply: serde_json::Value) -> (MockServer, Github) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let github = Github::with_token(config(&host), Token::Fixed("t".into())).expect("a client");
    (server, github)
}

#[tokio::test]
async fn an_issue_on_a_board_reads_as_a_task_through_the_mapping() {
    let reply = serde_json::json!({ "data": { "search": { "nodes": [issue()] } } });
    let (_server, github) = source(reply).await;
    let tasks = github.list().await.expect("the search answers");
    let task = tasks.first().expect("one task");
    assert_eq!(task.short_id, "gh-haoov-groove-50");
    assert!(
        task.external_id.as_str().ends_with("/haoov/groove#50"),
        "the key names the host, the repo and the number: {}",
        task.external_id.as_str()
    );
    assert_eq!(task.title, "Harden Groove");
    assert_eq!(task.status, "In progress");
    assert_eq!(task.intent, Some(StatusIntent::InProgress));
    assert_eq!(task.priority, Some(Priority::High));
    assert_eq!(task.dates.due.map(|day| day.day), Some(30));
    assert_eq!(task.estimate, Some(6.5));
    assert_eq!(task.logged, Some(1.5));
    assert_eq!(task.branch_tag.as_deref(), Some("50"));
    assert_eq!(task.board.as_deref(), Some("Platform"));
}

#[tokio::test]
async fn a_read_brings_the_task_and_the_issue_body() {
    let reply = serde_json::json!({ "data": { "repository": { "issue": issue() } } });
    let (_server, github) = source(reply).await;
    let read = github.fetch(&key()).await.expect("the issue answers");
    assert_eq!(read.task.short_id, "gh-haoov-groove-50");
    assert_eq!(read.body, "the body");
}

#[tokio::test]
async fn an_issue_on_no_board_is_not_a_task() {
    let mut bare = issue();
    bare["projectItems"]["nodes"] = serde_json::json!([]);
    let reply = serde_json::json!({ "data": { "search": { "nodes": [bare] } } });
    let (_server, github) = source(reply).await;
    assert!(github.list().await.expect("the search answers").is_empty());
}

#[tokio::test]
async fn a_property_the_config_does_not_name_is_left_out() {
    let reply = serde_json::json!({ "data": { "search": { "nodes": [issue()] } } });
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let mut config = config(&format!("http://{}", server.address()));
    config.properties.estimate = None;
    config.properties.due = None;
    let github = Github::with_token(config, Token::Fixed("t".into())).expect("a client");
    let tasks = github.list().await.expect("the search answers");
    let task = tasks.first().expect("one task");
    assert_eq!(task.estimate, None, "no name, no estimate, no error");
    assert_eq!(task.dates.due, None);
    assert_eq!(
        task.dates.start.map(|day| day.day),
        Some(14),
        "the rest stands"
    );
}

#[tokio::test]
async fn logging_hours_adds_them_to_what_the_board_already_holds() {
    let reply = serde_json::json!({ "data": { "repository": { "issue": issue() } } });
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let github = Github::with_token(config(&host), Token::Fixed("t".into())).expect("a client");
    let whole = github
        .log_hours(&key(), 0.5)
        .await
        .expect("the write lands");
    assert_eq!(whole, 2.0, "1.5 already spent and half an hour more");
    let sent: Vec<serde_json::Value> = server
        .received_requests()
        .await
        .expect("the calls")
        .iter()
        .map(|call| call.body_json().expect("json"))
        .collect();
    let write = sent.last().expect("the mutation");
    assert_eq!(write["variables"]["field"], "FIELD_SPENT");
    assert_eq!(write["variables"]["item"], "ITEM_1");
    assert_eq!(write["variables"]["project"], "BOARD_1");
    assert_eq!(write["variables"]["value"], 2.0);
}

#[tokio::test]
async fn a_status_is_written_as_the_option_the_map_names() {
    let reply = serde_json::json!({ "data": { "repository": { "issue": issue() } } });
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply))
        .mount(&server)
        .await;
    let host = format!("http://{}", server.address());
    let github = Github::with_token(config(&host), Token::Fixed("t".into())).expect("a client");
    let label = github
        .set_status(&key(), StatusIntent::Done)
        .await
        .expect("the write lands");
    assert_eq!(label, "Done", "the first name the map gives that intent");
    let sent: Vec<serde_json::Value> = server
        .received_requests()
        .await
        .expect("the calls")
        .iter()
        .map(|call| call.body_json().expect("json"))
        .collect();
    let write = sent.last().expect("the mutation");
    assert_eq!(write["variables"]["field"], "FIELD_STATUS");
    assert_eq!(write["variables"]["option"], "OPT_DONE");
}

#[tokio::test]
async fn a_status_the_board_does_not_offer_is_refused() {
    let mut bare = issue();
    bare["projectItems"]["nodes"][0]["project"]["fields"]["nodes"][1]["options"] =
        serde_json::json!([{ "id": "OPT_TODO", "name": "Todo" }]);
    let reply = serde_json::json!({ "data": { "repository": { "issue": bare } } });
    let (_server, github) = source(reply).await;
    let refused = github
        .set_status(&key(), StatusIntent::Done)
        .await
        .expect_err("it is refused");
    assert!(refused.to_string().contains("Done"), "{refused}");
}

#[tokio::test]
async fn a_board_without_the_hours_field_says_so_and_writes_nothing() {
    let mut bare = issue();
    bare["projectItems"]["nodes"][0]["project"]["fields"]["nodes"] = serde_json::json!([]);
    let reply = serde_json::json!({ "data": { "repository": { "issue": bare } } });
    let (_server, github) = source(reply).await;
    let key = groove_types::TaskKey::Github {
        host: "github.com".into(),
        owner: "haoov".into(),
        repo: "groove".into(),
        number: 50,
    };
    let refused = github
        .log_hours(&key, 1.0)
        .await
        .expect_err("it is refused");
    assert!(refused.to_string().contains("Spent"), "{refused}");
}

#[tokio::test]
async fn what_github_refuses_comes_back_as_the_reason_it_gave() {
    let reply = serde_json::json!({ "errors": [{ "message": "Bad credentials" }] });
    let (_server, github) = source(reply).await;
    let refused = github.list().await.expect_err("the query is refused");
    assert!(refused.to_string().contains("Bad credentials"), "{refused}");
}
