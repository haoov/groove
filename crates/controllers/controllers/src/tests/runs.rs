//! Routines at work: on the session an event is about, once, not selected, with auto-approve on.

use groove_agent_service::routines::{Listed, parse};
use groove_types::{AgentStatus, SessionId};

use crate::tests::fixture::{fresh, until};
use crate::{AppState, Command, Services, SyncSpawner, agent, config, dispatch, routine, session};

const NOTES: &str = "---\nskills: groove:fix-notes\nkind: bound\non: agent-finished\n---\n";
const DIGEST: &str =
    "---\nskills: platform:follow-ups\nkind: standalone\non: task-moved\n---\nList them.\n";

fn listed(state: &mut AppState, id: &str, text: &str) {
    let name = id.trim_start_matches("user:");
    state.agent.routines.push(Listed {
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
        s.agent.terminal(&id).is_some()
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

fn screen(state: &AppState, id: &SessionId, row: usize) -> String {
    let terminal = state.agent.terminal(id).expect("a terminal");
    terminal.screen().line(row).to_string()
}

fn task(short_id: &str, status: &str) -> groove_types::Task {
    groove_types::Task {
        external_id: groove_types::ExternalId::new(format!("notion:{short_id}")),
        short_id: short_id.into(),
        title: "a task".into(),
        status: status.into(),
        intent: None,
        priority: None,
        dates: groove_types::TaskDates::default(),
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
    until(&spawner, &services, &mut state, |s| {
        screen(s, &away, 0).starts_with("ready")
    });
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
            .agent
            .runs
            .running
            .iter()
            .all(|run| run.session == away)
    );
    assert_eq!(state.agent.runs.running.len(), 1);
    until(&spawner, &services, &mut state, |s| {
        screen(s, &away, 2) == "got /groove:fix-notes"
    });

    let run = &mut state.agent.runs.running[0];
    run.went = true;
    status(&mut state, &away, AgentStatus::Done { seen: false });
    routine::watch(&mut state, &services, &spawner);
    assert!(
        state.agent.runs.running.is_empty(),
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
    assert!(state.agent.runs.running.is_empty(), "once a session");

    state.session.selected = Some(away.clone());
    routine::watch(&mut state, &services, &spawner);
    state.session.selected = Some(here);
    routine::watch(&mut state, &services, &spawner);
    assert!(
        !state.agent.runs.queued_since_seen("user:notes", &away),
        "looked at, its count is back to zero"
    );
}

#[test]
fn a_task_that_moves_starts_a_standalone_routine_afresh_with_its_words() {
    let (home, spawner, services, mut state) = fresh();
    listed(&mut state, "user:digest", DIGEST);
    on("user:digest", &mut state, &services, &spawner);
    let id = state
        .session
        .routine_session("user:digest")
        .expect("its session");
    assert_ne!(
        state.session.selected.as_ref(),
        Some(&id),
        "switched on, it is not selected"
    );
    until(&spawner, &services, &mut state, |s| {
        s.agent.terminal(&id).is_some()
    });
    auto(&mut state, &id, true);

    state.task.tasks = vec![task("T-1", "In progress")];
    state.task.reading = true;
    routine::watch(&mut state, &services, &spawner);
    state.task.reading = false;
    routine::watch(&mut state, &services, &spawner);
    assert!(
        state.agent.runs.running.is_empty(),
        "a read alone moves nothing"
    );
    state.task.tasks[0].status = "Done".into();
    routine::watch(&mut state, &services, &spawner);

    let first = "ready Routine `digest`, started by task-moved.";
    until(&spawner, &services, &mut state, |s| {
        s.agent
            .terminal(&id)
            .is_some_and(|t| t.screen().line(0) == first)
    });
    let thread = home.path().join(format!("data/agent-launch/{id}.thread"));
    assert!(
        thread.is_file(),
        "each run starts a conversation of its own"
    );
}

#[test]
fn a_standalone_routine_s_button_runs_it_whatever_the_rules_but_a_bound_one_has_none() {
    let (_home, spawner, services, mut state) = fresh();
    listed(&mut state, "user:digest", DIGEST);
    listed(&mut state, "user:notes", NOTES);
    on("user:digest", &mut state, &services, &spawner);
    let id = state
        .session
        .routine_session("user:digest")
        .expect("its session");
    until(&spawner, &services, &mut state, |s| {
        s.agent.terminal(&id).is_some()
    });
    let run = |id: &str| Command::Agent(agent::Command::RunRoutine { id: id.into() });
    dispatch(run("user:digest"), &mut state, &services, &spawner);
    let first = "ready Routine `digest`, started by its button.";
    until(&spawner, &services, &mut state, |s| {
        s.agent
            .terminal(&id)
            .is_some_and(|t| t.screen().line(0) == first)
    });

    explorer("here", &mut state, &services, &spawner);
    dispatch(run("user:notes"), &mut state, &services, &spawner);
    assert_eq!(state.agent.runs.running.len(), 1, "only the digest runs");
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
    assert!(state.agent.runs.running.is_empty() && state.agent.runs.waiting.is_empty());
    assert!(!state.agent.runs.queued_since_seen("user:notes", &away));
}
