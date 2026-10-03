//! Action routines: Groove does the work itself, with no agent of the routine's own.

use groove_agent_service::routines::{Listed, parse};
use groove_types::{Day, Timestamp};

use crate::tests::fixture::{fresh, until};
use crate::{AppState, Command, agent, dispatch, session};

const START_DUE: &str = "---\nkind: action\ndo: start-due\non: tasks-read\n---\n";

/// A task of Up Next, due `due` days from today at `hours` of estimate, started `start` days ago.
fn task(short_id: &str, due: Option<i64>, hours: f32, start: Option<i64>) -> groove_types::Task {
    let today = Timestamp::now().day().days();
    groove_types::Task {
        external_id: groove_types::ExternalId::new(format!("notion:{short_id}")),
        short_id: short_id.into(),
        title: short_id.into(),
        status: "Ready".into(),
        intent: None,
        priority: None,
        dates: groove_types::TaskDates {
            start: start.map(|ago| Day::from_days(today - ago)),
            due: due.map(|days| Day::from_days(today + days)),
            duration_days: None,
        },
        estimate: Some(hours),
        logged: None,
        synced_at: Timestamp::now(),
        provider: groove_types::ProviderId::Notion,
        url: None,
        project: None,
        branch_tag: None,
    }
}

fn opened(state: &AppState, short_id: &str) -> bool {
    let id = groove_types::ExternalId::new(format!("notion:{short_id}"));
    state.session.working(&id).is_some()
}

#[test]
fn start_due_opens_the_tasks_that_must_start_today_with_start_task_and_keeps_the_selection() {
    let (_home, spawner, services, mut state) = fresh();
    state.agent.routines = vec![Listed {
        id: "user:start-due".into(),
        read: parse("user:start-due", "start-due", START_DUE),
    }];
    let open = session::Command::OpenExplorer {
        title: Some("here".into()),
    };
    dispatch(Command::Session(open), &mut state, &services, &spawner);
    let here = state.session.selected.clone().expect("a session");
    state.task.tasks = vec![
        task("PRESSED", Some(1), 16.0, None),
        task("LATE", Some(-2), 0.0, None),
        task("STARTED", None, 4.0, Some(1)),
        task("ROOMY", Some(10), 8.0, None),
        task("UNDATED", None, 8.0, None),
    ];
    let run = agent::Command::RunRoutine {
        id: "user:start-due".into(),
    };
    dispatch(Command::Agent(run), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);

    for due in ["PRESSED", "LATE", "STARTED"] {
        assert!(opened(&state, due), "{due} must start today");
    }
    for not in ["ROOMY", "UNDATED"] {
        assert!(!opened(&state, not), "{not} can wait");
    }
    assert_eq!(
        state.session.selected,
        Some(here),
        "the user stays where they were"
    );
    let notes: Vec<&str> = state.notes.iter().map(|one| one.what.as_str()).collect();
    assert!(
        notes.contains(&"start-due opened PRESSED, LATE, STARTED"),
        "{notes:?}"
    );
    let pressed = groove_types::ExternalId::new("notion:PRESSED");
    let id = state.session.working(&pressed).unwrap().session.id.clone();
    until(&spawner, &services, &mut state, |s| {
        s.agent
            .terminal(&id)
            .is_some_and(|t| t.screen().line(0) == "ready /groove:start-task")
    });
}
