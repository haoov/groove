//! The routines' rules: what an event between two looks is, where it runs, how many run at once.

use groove_types::{
    AgentStatus, CiState, ExternalId, Routine, RoutineKind, RoutinesConfig, SessionId, Timestamp,
    Trigger, WorktreeId,
};

use crate::runs::{Ending, Fired, Place, Run, Runs, Seen, Sessions};

fn id(one: &str) -> SessionId {
    SessionId::new(one)
}

/// One worktree `w` of session `s` on branch `fix/x`, as a look sees it.
fn seen() -> Seen {
    let mut seen = Seen::default();
    seen.owners
        .insert(WorktreeId::new("w"), (id("s"), "fix/x".into()));
    seen
}

fn triggers(fired: &[Fired]) -> Vec<Trigger> {
    fired.iter().map(|one| one.trigger).collect()
}

#[test]
fn a_run_gone_red_and_reviews_given_on_a_known_mr_fire_on_its_session() {
    let mut before = seen();
    before.ci.insert(WorktreeId::new("w"), CiState::Running);
    before.changes.insert(WorktreeId::new("w"), false);
    before.commented.insert(WorktreeId::new("w"), false);
    let mut now = before.clone();
    now.ci.insert(WorktreeId::new("w"), CiState::Failed);
    now.changes.insert(WorktreeId::new("w"), true);
    now.commented.insert(WorktreeId::new("w"), true);
    let fired = before.fired(&now);
    let wanted = [
        Trigger::CiFailed,
        Trigger::ChangesRequested,
        Trigger::ReviewCommented,
    ];
    assert_eq!(triggers(&fired), wanted);
    assert!(fired.iter().all(|one| one.session == Some(id("s"))));
    assert_eq!(fired[0].about, "CI failed on fix/x");
}

#[test]
fn what_was_already_so_at_the_first_look_fires_nothing() {
    let mut before = seen();
    before.asked = None;
    let mut now = before.clone();
    now.ci.insert(WorktreeId::new("w"), CiState::Failed);
    now.changes.insert(WorktreeId::new("w"), true);
    now.asked = Some(["review-1".to_string()].into());
    assert!(
        before.fired(&now).is_empty(),
        "unread before, nothing changed"
    );
}

#[test]
fn a_new_review_in_the_queue_and_a_finished_agent_fire() {
    let mut before = seen();
    before.asked = Some(Default::default());
    before.working.insert(id("s"));
    let mut now = seen();
    now.asked = Some(["review-1".to_string()].into());
    now.done.insert(id("s"));
    let fired = before.fired(&now);
    assert_eq!(
        triggers(&fired),
        [Trigger::ReviewAsked, Trigger::AgentFinished]
    );
    assert_eq!(fired[0].session, Some(id("review-1")));
}

#[test]
fn a_task_moves_only_between_two_reads_after_the_first() {
    let task = |status: &str| {
        (
            ExternalId::new("t"),
            ("T-1".to_string(), status.to_string()),
        )
    };
    let mut before = Seen {
        reading: true,
        ..Seen::default()
    };
    before.tasks.extend([task("Todo")]);
    let mut now = Seen::default();
    now.tasks.extend([task("Doing")]);
    assert_eq!(
        triggers(&before.fired(&now)),
        [Trigger::TasksRead],
        "the first read"
    );
    let read = Seen {
        tasks_read: true,
        ..now.clone()
    };
    let mut later = read.clone();
    later.tasks.extend([task("Done")]);
    let fired = read.fired(&later);
    assert_eq!(triggers(&fired), [Trigger::TaskMoved]);
    assert_eq!(fired[0].about, "T-1 moved to Done");
}

fn routine(name: &str, kind: RoutineKind, on: Trigger) -> Routine {
    Routine {
        id: format!("user:{name}"),
        name: name.into(),
        description: String::new(),
        skills: vec!["groove:fix-ci".into()],
        kind,
        on: vec![on],
        words: String::new(),
        action: None,
    }
}

fn switched(routines: &[&Routine]) -> RoutinesConfig {
    RoutinesConfig {
        on: routines.iter().map(|one| one.id.clone()).collect(),
        ..RoutinesConfig::default()
    }
}

fn place(one: &str, auto_approve: bool) -> Place {
    Place {
        id: id(one),
        routine: None,
        auto_approve,
    }
}

fn red(on: &str) -> Fired {
    Fired {
        trigger: Trigger::CiFailed,
        session: Some(id(on)),
        about: String::new(),
    }
}

#[test]
fn a_bound_routine_runs_once_where_writes_run_unasked_and_never_on_the_selected_session() {
    let fix = routine("fix", RoutineKind::Bound, Trigger::CiFailed);
    let (held, routines) = (switched(&[&fix]), vec![fix.clone()]);
    let sessions = Sessions {
        open: vec![
            place("away", true),
            place("asking", false),
            place("here", true),
        ],
        selected: Some(id("here")),
    };
    let mut runs = Runs::default();
    for on in ["away", "asking", "here", "away"] {
        runs.fire(&red(on), &routines, &held, &sessions);
    }
    let queued: Vec<&SessionId> = runs.waiting.iter().map(|one| &one.session).collect();
    assert_eq!(
        queued,
        [&id("away")],
        "once, unasked, and not where the user is"
    );

    runs.waiting.clear();
    runs.seen_session(&id("away"));
    runs.fire(&red("away"), &routines, &held, &sessions);
    assert_eq!(runs.waiting.len(), 1, "looked at, it may run again");
}

#[test]
fn a_quiet_trigger_or_a_routine_switched_off_starts_nothing() {
    let fix = routine("fix", RoutineKind::Bound, Trigger::CiFailed);
    let mut held = switched(&[&fix]);
    held.quiet.insert(fix.id.clone(), vec![Trigger::CiFailed]);
    let sessions = Sessions {
        open: vec![place("away", true)],
        selected: None,
    };
    let mut runs = Runs::default();
    runs.fire(&red("away"), std::slice::from_ref(&fix), &held, &sessions);
    runs.fire(
        &red("away"),
        std::slice::from_ref(&fix),
        &RoutinesConfig::default(),
        &sessions,
    );
    assert!(runs.waiting.is_empty());
}

#[test]
fn no_more_run_together_than_the_cap_and_a_busy_agent_waits() {
    let mut runs = Runs::default();
    for one in ["a", "b", "c"] {
        runs.queue(Run::new("user:fix", id(one), Some(Trigger::CiFailed), ""));
    }
    let now = Timestamp::new(1000);
    let working = |one: &SessionId| (one == &id("b")).then_some(AgentStatus::Working);
    let started = runs.due(2, |_| Some(RoutineKind::Bound), working, now);
    let on: Vec<&SessionId> = started.iter().map(|one| &one.session).collect();
    assert_eq!(
        on,
        [&id("a"), &id("c")],
        "b's agent is busy; the cap holds two"
    );
    assert_eq!(runs.waiting.len(), 1);
}

#[test]
fn a_run_ends_when_its_agent_worked_then_stopped_went_or_never_started() {
    let mut runs = Runs::default();
    for one in ["done", "gone", "stalled"] {
        let run = Run::new("user:fix", id(one), None, "");
        runs.start_now(run, Timestamp::new(0));
    }
    let working = |_: &SessionId| Some(AgentStatus::Working);
    assert!(
        runs.finished(working, Timestamp::new(10)).is_empty(),
        "all three work"
    );
    runs.running[2].went = false;
    let status = |one: &SessionId| match one.as_str() {
        "done" => Some(AgentStatus::Done { seen: false }),
        "gone" => Some(AgentStatus::Exited { code: 1 }),
        _ => Some(AgentStatus::Idle),
    };
    let ended = runs.finished(status, Timestamp::new(121));
    let how: Vec<Ending> = ended.iter().map(|(_, how)| *how).collect();
    assert_eq!(how, [Ending::Done, Ending::Gone, Ending::Stalled]);
    assert!(runs.running.is_empty());
}

#[test]
fn a_session_that_goes_takes_its_runs_with_it() {
    let mut runs = Runs::default();
    runs.queue(Run::new("user:fix", id("s"), None, ""));
    runs.start_now(Run::new("user:other", id("s"), None, ""), Timestamp::new(0));
    runs.drop_session(&id("s"));
    assert!(runs.waiting.is_empty() && runs.running.is_empty());
    assert!(!runs.queued_since_seen("user:fix", &id("s")));
}
