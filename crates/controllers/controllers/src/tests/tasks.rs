//! The board's own reading: what a source answers lands in the slice.

use groove_types::{Priority, StatusIntent};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::fixture::{services, state, until};
use crate::{Command as Cmd, SyncSpawner, dispatch, task};

fn issue() -> serde_json::Value {
    serde_json::json!({
        "number": 50,
        "title": "Harden Groove",
        "url": "https://example.test/haoov/groove/issues/50",
        "body": "Close the gates.",
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
    })
}

/// One answer for both queries: the search the list makes, the issue a read makes.
fn issues() -> serde_json::Value {
    serde_json::json!({ "data": {
        "search": { "nodes": [issue()] },
        "repository": { "issue": issue() }
    }})
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

#[test]
fn opening_a_task_starts_a_session_that_works_it() {
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

    dispatch(
        Cmd::Task(task::Command::Open {
            short_id: "gh-haoov-groove-50".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.agent
            .agent(&groove_types::SessionId::new("gh-haoov-groove-50"))
            .is_some()
    });
    let open = state.session.open.first().expect("a session on the rail");
    assert_eq!(open.session.title, "Harden Groove");
    assert!(
        matches!(&open.session.kind, groove_types::SessionKind::Task { external_id }
            if external_id.as_str().ends_with("/haoov/groove#50")),
        "it works that task: {:?}",
        open.session.kind
    );
    assert_eq!(state.session.selected.as_ref(), Some(&open.session.id));
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn the_task_a_session_works_arrives_with_its_body() {
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

    dispatch(
        Cmd::Task(task::Command::Open {
            short_id: "gh-haoov-groove-50".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.task.body("gh-haoov-groove-50").is_some()
    });
    assert_eq!(
        state.task.body("gh-haoov-groove-50"),
        Some("Close the gates.")
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn opening_a_task_that_is_already_open_selects_its_session() {
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
    let open = || {
        Cmd::Task(task::Command::Open {
            short_id: "gh-haoov-groove-50".into(),
        })
    };
    dispatch(open(), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        !s.session.open.is_empty()
    });

    dispatch(open(), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert_eq!(state.session.open.len(), 1, "one session, not two");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

/// The window focused on the session, with input just now.
fn at_work(state: &mut crate::AppState) {
    state.focused = true;
    state.acted_at = groove_types::Timestamp::now();
}

#[test]
fn the_clock_credits_the_task_the_window_is_working() {
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
    dispatch(
        Cmd::Task(task::Command::Open {
            short_id: "gh-haoov-groove-50".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.session.open.is_empty()
    });
    at_work(&mut state);

    let now = groove_types::Timestamp::now();
    task::time::tick(&mut state, &services, &spawner, now);
    let later = groove_types::Timestamp::new(now.seconds() + 90);
    task::time::tick(&mut state, &services, &spawner, later);
    until(&spawner, &services, &mut state, |s| !s.task.time.is_empty());

    let id = groove_types::ExternalId::new(format!("{host}/haoov/groove#50"));
    let measured = state.task.measured(&id).expect("the task is measured");
    assert_eq!(measured.tracked_seconds, 90);
    assert_eq!(measured.unlogged_seconds, 90, "and none of it is logged");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn an_explorer_is_measured_against_no_task() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    dispatch(
        Cmd::Session(crate::session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    at_work(&mut state);
    let now = groove_types::Timestamp::now();
    task::time::tick(&mut state, &services, &spawner, now);
    task::time::tick(
        &mut state,
        &services,
        &spawner,
        groove_types::Timestamp::new(now.seconds() + 90),
    );
    spawner.drain(&mut state, &services);
    assert!(state.task.time.is_empty(), "nothing was measured");
}
