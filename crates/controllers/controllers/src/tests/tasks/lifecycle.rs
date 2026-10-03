//! The task's own lifecycle: the clock on it, its status, and finishing it.

use groove_types::StatusIntent;

use super::{answering, source};
use crate::tests::fixture::until;
use crate::{Command as Cmd, dispatch, task};

/// The window focused on the session, with input just now.
fn at_work(state: &mut crate::AppState) {
    state.focused = true;
    state.acted_at = groove_types::Timestamp::now();
}

#[test]
fn the_clock_credits_the_task_the_window_is_working() {
    let (_runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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

#[test]
fn opening_a_task_tells_the_source_it_is_in_progress() {
    let (runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    let task = state.task.tasks.first_mut().expect("the task");
    task.status = "Todo".into();
    task.intent = Some(StatusIntent::Ready);

    dispatch(
        Cmd::Task(task::Command::Open {
            short_id: "gh-haoov-groove-50".into(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    let sent: Vec<serde_json::Value> = runtime
        .block_on(server.received_requests())
        .expect("the calls")
        .iter()
        .map(|call| call.body_json().expect("json"))
        .collect();
    assert!(
        sent.iter()
            .any(|call| call["variables"]["option"] == "OPT_DOING"),
        "the option the status map points at: {sent:?}"
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_task_already_in_progress_is_not_written_again() {
    let (runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    spawner.drain(&mut state, &services);
    let sent: Vec<serde_json::Value> = runtime
        .block_on(server.received_requests())
        .expect("the calls")
        .iter()
        .map(|call| call.body_json().expect("json"))
        .collect();
    assert!(
        !sent.iter().any(|call| call["query"]
            .as_str()
            .is_some_and(|query| query.starts_with("mutation"))),
        "the source already says so: {sent:?}"
    );
}

#[test]
fn finishing_a_task_tells_the_source_then_takes_the_session_away() {
    let (runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    let session = state.session.open[0].session.id.clone();

    dispatch(
        Cmd::Task(task::Command::Finish {
            session: session.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.session.get(&session).is_none()
    });
    let sent: Vec<serde_json::Value> = runtime
        .block_on(server.received_requests())
        .expect("the calls")
        .iter()
        .map(|call| call.body_json().expect("json"))
        .collect();
    assert!(
        sent.iter()
            .any(|call| call["variables"]["option"] == "OPT_DONE"),
        "the option the map's done points at: {sent:?}"
    );
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_session_that_works_no_task_is_not_finished() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    dispatch(
        Cmd::Session(crate::session::Command::OpenExplorer { title: None }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    let session = state.session.open[0].session.id.clone();
    dispatch(
        Cmd::Task(task::Command::Finish {
            session: session.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(state.session.get(&session).is_some(), "it stands");
}

#[test]
fn deleting_a_task_session_here_says_nothing_to_the_source() {
    let (runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
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
    let session = state.session.open[0].session.id.clone();
    let before = runtime
        .block_on(server.received_requests())
        .expect("the calls")
        .len();

    dispatch(
        Cmd::Session(crate::session::Command::DeleteLocal {
            session: session.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.session.get(&session).is_none()
    });
    let sent: Vec<serde_json::Value> = runtime
        .block_on(server.received_requests())
        .expect("the calls")
        .iter()
        .skip(before)
        .map(|call| call.body_json().expect("json"))
        .collect();
    assert!(
        !sent.iter().any(|call| call["query"]
            .as_str()
            .is_some_and(|query| query.starts_with("mutation"))),
        "nothing was written: {sent:?}"
    );
}

#[test]
fn a_task_past_its_due_date_asks_for_the_user() {
    let (_runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let host = format!("http://{}", server.address());
    let mut config: groove_types::GithubConfig =
        serde_json::from_value(source(&host)).expect("the source");
    config.properties.due = Some("Due".into());
    state.config.config.as_mut().expect("a config").github = Some(config);
    dispatch(
        Cmd::Task(task::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        !s.task.tasks.is_empty()
    });

    let id = state.task.tasks[0].external_id.clone();
    assert!(!state.task.asks(&id), "nothing is due yet");
    let task = state.task.tasks.first_mut().expect("the task");
    task.dates.due = Some(groove_types::Timestamp::now().day().plus_days(-2));
    task::attention::reread(&mut state, groove_types::Timestamp::now());
    assert_eq!(
        state.task.needs(&id),
        [groove_types::Attention::Overdue { by_days: 2 }],
        "two days past it"
    );
}

#[test]
fn a_source_that_fails_to_finish_the_task_keeps_its_session_and_says_why() {
    let (runtime, server) = answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let host = format!("http://{}", server.address());
    state.config.config.as_mut().expect("a config").github =
        Some(serde_json::from_value(source(&host)).expect("the source"));
    let load = Cmd::Task(task::Command::Load);
    dispatch(load, &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        !s.task.tasks.is_empty()
    });
    let open = task::Command::Open {
        short_id: "gh-haoov-groove-50".into(),
    };
    dispatch(Cmd::Task(open), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        !s.session.open.is_empty()
    });
    let session = state.session.open[0].session.id.clone();
    state.errors.clear();

    runtime.block_on(async {
        server.reset().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;
    });
    let finish = task::Command::Finish {
        session: session.clone(),
    };
    dispatch(Cmd::Task(finish), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.pending.is_empty() && !s.errors.is_empty()
    });
    spawner.drain(&mut state, &services);
    assert!(state.session.get(&session).is_some(), "the session stays");
}
