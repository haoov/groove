//! The board's own reading: what a source answers lands in the slice.

use groove_types::{Priority, StatusIntent};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::fixture::{services, state, until};
use crate::{Command as Cmd, SyncSpawner, dispatch, task};

fn issues() -> serde_json::Value {
    serde_json::json!({ "data": { "search": { "nodes": [{
        "number": 50,
        "title": "Harden Groove",
        "url": "https://example.test/haoov/groove/issues/50",
        "body": "",
        "repository": { "name": "groove", "owner": { "login": "haoov" } },
        "projectItems": { "nodes": [{
            "project": { "title": "Platform" },
            "fieldValues": { "nodes": [
                { "__typename": "ProjectV2ItemFieldSingleSelectValue",
                  "name": "In progress", "field": { "name": "Status" } },
                { "__typename": "ProjectV2ItemFieldSingleSelectValue",
                  "name": "P1", "field": { "name": "Priority" } }
            ]}
        }]}
    }]}}})
}

/// The config a source needs: the host, and what its fields are called.
fn source(host: &str) -> serde_json::Value {
    serde_json::json!({
        "host": host,
        "token": "t",
        "properties": { "status": "Status", "priority": "Priority" },
        "status_map": { "ready": ["Todo"], "in_progress": ["In progress"], "done": ["Done"] },
        "priority_map": { "high": ["P1"], "medium": ["P2"], "low": ["P3"] }
    })
}

/// A server answering the GraphQL call, on a runtime of its own that outlives it.
fn answering() -> (tokio::runtime::Runtime, MockServer) {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    let server = runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issues()))
            .mount(&server)
            .await;
        server
    });
    (runtime, server)
}

#[test]
fn the_tasks_a_source_answers_with_land_in_the_slice() {
    let (_runtime, server) = answering();
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let host = format!("http://{}", server.address());
    state.config.config.as_mut().expect("a config").github =
        Some(serde_json::from_value(source(&host)).expect("the source"));

    dispatch(
        Cmd::Task(task::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.task.tasks.is_empty()
    });
    let task = state.task.get("gh-haoov-groove-50").expect("the task");
    assert_eq!(task.title, "Harden Groove");
    assert_eq!(task.intent, Some(StatusIntent::InProgress));
    assert_eq!(task.priority, Some(Priority::High));
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn nothing_is_read_while_no_source_is_configured() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());

    dispatch(
        Cmd::Task(task::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.task.tasks.is_empty());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(!state.task.reading, "and nothing is left in flight");
}
