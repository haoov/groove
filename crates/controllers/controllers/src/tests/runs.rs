//! Routines at work: on the session an event is about, once, not selected, with auto-approve on.

use groove_routine_service::routines::{Listed, parse};
use groove_types::{AgentStatus, SessionId};

use crate::tests::fixture::{fresh, until};
use crate::{AppState, Command, Services, SyncSpawner, agent, config, dispatch, routine, session};

const NOTES: &str = "---\nskills: groove:fix-notes\nkind: bound\non: agent-finished\n---\n";

fn listed(state: &mut AppState, id: &str, text: &str) {
    let name = id.trim_start_matches("user:");
    state.routine.routines.push(Listed {
        id: id.into(),
        read: parse(id, name, text),
    });
}

fn on(id: &str, state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    let switch = config::Command::SwitchRoutine {
        id: id.into(),
        on: true,
    };
    dispatch(Command::Config(switch), state, services, spawner);
    spawner.drain(state, services);
}

fn explorer(
    title: &str,
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> SessionId {
    let open = session::Command::OpenExplorer {
        title: Some(title.into()),
    };
    dispatch(Command::Session(open), state, services, spawner);
    let id = state.session.selected.clone().expect("a session");
    until(spawner, services, state, |s| {
        s.agent.stream(&id).is_some()
    });
    id
}

fn auto(state: &mut AppState, id: &SessionId, on: bool) {
    state.session.get_mut(id).expect("open").state.auto_approve = on;
}

fn status(state: &mut AppState, id: &SessionId, to: AgentStatus) {
    let agent = state.agent.agents.iter_mut().find(|(one, _)| one == id);
    agent.expect("its agent").1.activity.status = to;
}

fn heard(state: &AppState, id: &SessionId, words: &str) -> bool {
    let heard = crate::tests::fixture::heard(&state.env.home, id.as_str());
    heard.is_some_and(|(_, said)| said.iter().any(|one| one.contains(words)))
}

fn due_today(short_id: &str) -> groove_types::Task {
    let today = groove_types::Timestamp::now().day();
    groove_types::Task {
        external_id: groove_types::ExternalId::new(format!("notion:{short_id}")),
        short_id: short_id.into(),
        title: "a task".into(),
        status: "Ready".into(),
        intent: None,
        priority: None,
        dates: groove_types::TaskDates {
            due: Some(today),
            ..groove_types::TaskDates::default()
        },
        estimate: None,
        logged: None,
        synced_at: groove_types::Timestamp::now(),
        provider: groove_types::ProviderId::Notion,
        url: None,
        project: None,
        branch_tag: None,
    }
}

#[test]
fn an_agent_that_finishes_runs_a_bound_routine_once_but_never_on_the_selected_session() {
    let (_home, spawner, services, mut state) = fresh();
    listed(&mut state, "user:notes", NOTES);
    on("user:notes", &mut state, &services, &spawner);
    let away = explorer("away", &mut state, &services, &spawner);
    let here = explorer("here", &mut state, &services, &spawner);
    for id in [&away, &here] {
        auto(&mut state, id, true);
    }
    routine::watch(&mut state, &services, &spawner);
    for id in [&away, &here] {
        status(&mut state, id, AgentStatus::Working);
    }
    routine::watch(&mut state, &services, &spawner);
    for id in [&away, &here] {
        status(&mut state, id, AgentStatus::Done { seen: false });
    }
    routine::watch(&mut state, &services, &spawner);
    assert!(
        state
            .routine
            .runs
            .running
            .iter()
            .all(|run| run.session == away)
    );
    assert_eq!(state.routine.runs.running.len(), 1);
    until(&spawner, &services, &mut state, |s| {
        heard(s, &away, "/groove:fix-notes")
    });

    let run = &mut state.routine.runs.running[0];
    run.went = true;
    status(&mut state, &away, AgentStatus::Done { seen: false });
    routine::watch(&mut state, &services, &spawner);
    assert!(
        state.routine.runs.running.is_empty(),
        "its agent stopped, the run is over"
    );
    spawner.drain(&mut state, &services);
    let fed: Vec<_> = state
        .session
        .feed
        .iter()
        .filter(|one| one.session == away)
        .collect();
    let said: Vec<&str> = fed.iter().map(|one| one.subject.as_str()).collect();
    assert!(said.contains(&"notes · agent-finished"), "{said:?}");
    assert!(said.contains(&"notes · done"), "{said:?}");
    status(&mut state, &away, AgentStatus::Working);
    routine::watch(&mut state, &services, &spawner);
    status(&mut state, &away, AgentStatus::Done { seen: false });
    routine::watch(&mut state, &services, &spawner);
    assert!(state.routine.runs.running.is_empty(), "once a session");

    state.session.selected = Some(away.clone());
    routine::watch(&mut state, &services, &spawner);
    state.session.selected = Some(here);
    routine::watch(&mut state, &services, &spawner);
    assert!(
        !state.routine.runs.queued_since_seen("user:notes", &away),
        "looked at, its count is back to zero"
    );
}

#[test]
fn a_bound_routine_has_no_button() {
    let (_home, spawner, services, mut state) = fresh();
    listed(&mut state, "user:notes", NOTES);
    explorer("here", &mut state, &services, &spawner);
    let run = agent::Command::RunRoutine {
        id: "user:notes".into(),
    };
    dispatch(Command::Agent(run), &mut state, &services, &spawner);
    assert!(state.routine.runs.running.is_empty());
    assert!(
        state
            .errors
            .iter()
            .any(|one| one.what.message.contains("bound"))
    );
}

#[test]
fn an_event_on_a_session_with_auto_approve_off_starts_nothing() {
    let (_home, spawner, services, mut state) = fresh();
    listed(&mut state, "user:notes", NOTES);
    on("user:notes", &mut state, &services, &spawner);
    let away = explorer("away", &mut state, &services, &spawner);
    explorer("here", &mut state, &services, &spawner);
    auto(&mut state, &away, false);
    routine::watch(&mut state, &services, &spawner);
    status(&mut state, &away, AgentStatus::Working);
    routine::watch(&mut state, &services, &spawner);
    status(&mut state, &away, AgentStatus::Done { seen: false });
    routine::watch(&mut state, &services, &spawner);
    assert!(state.routine.runs.running.is_empty() && state.routine.runs.waiting.is_empty());
    assert!(!state.routine.runs.queued_since_seen("user:notes", &away));
}

const DAILY: &str = "---\nkind: action\ndo: start-due\non: daily\n---\n";

#[test]
fn the_daily_routine_acts_on_the_day_s_first_look_and_not_again_that_day() {
    let (home, spawner, services, mut state) = fresh();
    listed(&mut state, "user:daily", DAILY);
    on("user:daily", &mut state, &services, &spawner);
    state.task.tasks = vec![due_today("T-1")];
    routine::watch(&mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert!(state.notes.is_empty(), "the rail is not back");

    (state.session.restored, state.skill.listed) = (true, true);
    routine::watch(&mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let said = |state: &AppState| {
        let notes = state
            .notes
            .iter()
            .filter(|one| one.what == "start-due opened T-1");
        notes.count()
    };
    assert_eq!(said(&state), 1, "{:?}", state.notes);
    let day = std::fs::read_to_string(home.path().join("data/routines/day"));
    assert!(day.is_ok_and(|one| !one.is_empty()), "the day written down");

    state.routine.runs.dawned = false;
    routine::watch(&mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert_eq!(said(&state), 1, "once a day");
}
