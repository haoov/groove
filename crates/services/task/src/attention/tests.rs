//! What a task's MR makes it ask for.

use std::collections::BTreeMap;

use groove_types::{
    Attention, CiState, ExternalId, MrFacts, MrState, ProviderId, Task, TaskDates, Thresholds,
    Timestamp,
};

use crate::folded;

const DAY: i64 = 86_400;

fn external() -> ExternalId {
    ExternalId::new("github.com/haoov/groove#50")
}

fn task() -> Task {
    Task {
        external_id: external(),
        short_id: "gh-haoov-groove-50".into(),
        title: "Harden Groove".into(),
        status: "In progress".into(),
        intent: None,
        priority: None,
        dates: TaskDates::default(),
        estimate: None,
        logged: None,
        synced_at: Timestamp::now(),
        provider: ProviderId::Github,
        url: None,
        board: None,
        branch_tag: None,
    }
}

fn now() -> Timestamp {
    Timestamp::new(20 * DAY)
}

/// What the task asks for, its MR standing as `facts` say.
fn needs(facts: MrFacts) -> Vec<Attention> {
    let by_task = BTreeMap::from([(external(), facts)]);
    let folded = folded(&[task()], &by_task, now(), &Thresholds::default());
    folded.get(&external()).cloned().unwrap_or_default()
}

#[test]
fn a_review_nobody_has_answered_wakes_the_task() {
    let facts = MrFacts {
        state: Some(MrState::Open),
        review_requested_at: Some(Timestamp::new(16 * DAY)),
        ..Default::default()
    };
    let since = Timestamp::new(16 * DAY);
    assert_eq!(needs(facts), [Attention::ReviewWaiting { since }]);
}

#[test]
fn a_run_that_failed_and_changes_asked_for_both_wake_it() {
    let facts = MrFacts {
        state: Some(MrState::Open),
        changes_requested_at: Some(Timestamp::new(18 * DAY)),
        ci: Some(CiState::Failed),
        ci_finished_at: Some(Timestamp::new(19 * DAY)),
        ..Default::default()
    };
    assert_eq!(
        needs(facts),
        [
            Attention::ChangesRequested {
                since: Timestamp::new(18 * DAY)
            },
            Attention::CiFailed {
                since: Some(Timestamp::new(19 * DAY))
            }
        ]
    );
}

#[test]
fn an_mr_approved_and_green_but_unmerged_wakes_it() {
    let facts = MrFacts {
        state: Some(MrState::Open),
        approved_at: Some(Timestamp::new(17 * DAY)),
        ci: Some(CiState::Success),
        ..Default::default()
    };
    let since = Timestamp::new(17 * DAY);
    assert_eq!(needs(facts), [Attention::ApprovedUnmerged { since }]);
    assert!(needs(MrFacts::default()).is_empty(), "no mr asks nothing");
}

#[test]
fn a_merged_mr_leaves_the_task_alone() {
    let facts = MrFacts {
        state: Some(MrState::Merged),
        approved_at: Some(Timestamp::new(10 * DAY)),
        ci: Some(CiState::Success),
        ..Default::default()
    };
    assert!(needs(facts).is_empty());
}

/// The task in a slice, read again with these sessions' days and MRs.
fn reread(
    started: Vec<(ExternalId, groove_types::Day)>,
    mrs: Vec<(ExternalId, MrFacts)>,
) -> crate::State {
    let mut state = crate::State {
        tasks: vec![task()],
        ..Default::default()
    };
    state.reread(started, mrs, now(), &Thresholds::default());
    state
}

#[test]
fn a_task_the_source_gives_no_start_starts_with_its_first_session() {
    let days = [5, 3, 9].map(|at| (external(), Timestamp::new(at * DAY).day()));
    let state = reread(days.to_vec(), Vec::new());
    assert_eq!(
        state.tasks[0].dates.start,
        Some(Timestamp::new(3 * DAY).day())
    );
}

#[test]
fn a_task_s_mrs_count_as_one() {
    let green = MrFacts {
        state: Some(MrState::Open),
        ci: Some(CiState::Success),
        ..Default::default()
    };
    let red = MrFacts {
        state: Some(MrState::Open),
        ci: Some(CiState::Failed),
        ci_finished_at: Some(Timestamp::new(19 * DAY)),
        ..Default::default()
    };
    let state = reread(Vec::new(), vec![(external(), green), (external(), red)]);
    let since = Some(Timestamp::new(19 * DAY));
    assert_eq!(
        state.needs(&external()),
        [Attention::CiFailed { since }],
        "the worst run"
    );
}
