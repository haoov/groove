//! What asks for attention on a task: its MR's facts, its dates, and the thresholds.

use crate::{Attention, CiState, MrFacts, MrState, TaskDates, Thresholds, Timestamp, attention};

pub(super) const DAY: i64 = 86_400;

#[test]
fn attention_rules_read_thresholds() {
    let now = Timestamp::new(20 * DAY);
    let t = Thresholds::default();
    let waiting = MrFacts {
        state: Some(MrState::Open),
        review_requested_at: Some(Timestamp::new(16 * DAY)),
        ..Default::default()
    };
    assert_eq!(
        attention(&waiting, &TaskDates::default(), now, &t),
        vec![Attention::ReviewWaiting {
            since: Timestamp::new(16 * DAY)
        }]
    );
    let fresh = MrFacts {
        review_requested_at: Some(Timestamp::new(19 * DAY)),
        ..waiting
    };
    assert!(attention(&fresh, &TaskDates::default(), now, &t).is_empty());

    let approved_red = MrFacts {
        state: Some(MrState::Open),
        approved_at: Some(Timestamp::new(10 * DAY)),
        ci: Some(CiState::Failed),
        ci_finished_at: Some(Timestamp::new(19 * DAY)),
        ..Default::default()
    };
    assert_eq!(
        attention(&approved_red, &TaskDates::default(), now, &t),
        vec![Attention::CiFailed {
            since: Some(Timestamp::new(19 * DAY))
        }]
    );
    let approved_green = MrFacts {
        ci: Some(CiState::Success),
        ..approved_red
    };
    assert_eq!(
        attention(&approved_green, &TaskDates::default(), now, &t),
        vec![Attention::ApprovedUnmerged {
            since: Timestamp::new(10 * DAY)
        }]
    );
    let merged = MrFacts {
        state: Some(MrState::Merged),
        ..approved_green
    };
    assert!(attention(&merged, &TaskDates::default(), now, &t).is_empty());
}

#[test]
fn due_dates_are_attention_too() {
    let now = Timestamp::new(20 * DAY);
    let t = Thresholds::default();
    let due_in = |days: i64| TaskDates {
        due: Some(now.day().plus_days(days)),
        ..Default::default()
    };
    assert_eq!(
        attention(&MrFacts::default(), &due_in(2), now, &t),
        vec![Attention::DueSoon { in_days: 2 }]
    );
    assert_eq!(
        attention(&MrFacts::default(), &due_in(-3), now, &t),
        vec![Attention::Overdue { by_days: 3 }]
    );
    assert!(attention(&MrFacts::default(), &due_in(5), now, &t).is_empty());
}

#[test]
fn two_mrs_of_one_task_read_as_one() {
    let one = MrFacts {
        state: Some(MrState::Merged),
        review_requested_at: Some(Timestamp::new(5 * DAY)),
        ci: Some(CiState::Success),
        ci_finished_at: Some(Timestamp::new(6 * DAY)),
        approved_at: Some(Timestamp::new(6 * DAY)),
        ..Default::default()
    };
    let two = MrFacts {
        state: Some(MrState::Open),
        review_requested_at: Some(Timestamp::new(3 * DAY)),
        ci: Some(CiState::Failed),
        ci_finished_at: Some(Timestamp::new(9 * DAY)),
        ..Default::default()
    };
    let both = one.and(two);
    assert_eq!(both.state, Some(MrState::Open), "one is still open");
    assert_eq!(both.review_requested_at, Some(Timestamp::new(3 * DAY)));
    assert_eq!(both.ci, Some(CiState::Failed), "the worst run");
    assert_eq!(both.ci_finished_at, Some(Timestamp::new(9 * DAY)));
    assert_eq!(both.approved_at, None, "only one of them is approved");

    let approved = MrFacts {
        approved_at: Some(Timestamp::new(4 * DAY)),
        ..two
    };
    assert_eq!(
        one.and(approved).approved_at,
        Some(Timestamp::new(6 * DAY)),
        "both approved, and the later one says when"
    );
}

#[test]
fn a_run_that_just_failed_is_left_alone_for_the_push_that_fixes_it() {
    let now = Timestamp::new(20 * DAY);
    let t = Thresholds::default();
    let minute = 60;
    let fresh = MrFacts {
        state: Some(MrState::Open),
        ci: Some(CiState::Failed),
        ci_finished_at: Some(Timestamp::new(now.seconds() - 2 * minute)),
        ..Default::default()
    };
    assert!(
        attention(&fresh, &TaskDates::default(), now, &t).is_empty(),
        "two minutes old, and the threshold is ten"
    );

    let settled = MrFacts {
        ci_finished_at: Some(Timestamp::new(now.seconds() - 11 * minute)),
        ..fresh
    };
    assert_eq!(
        attention(&settled, &TaskDates::default(), now, &t),
        vec![Attention::CiFailed {
            since: settled.ci_finished_at
        }]
    );

    let undated = MrFacts {
        ci_finished_at: None,
        ..fresh
    };
    assert_eq!(
        attention(&undated, &TaskDates::default(), now, &t),
        vec![Attention::CiFailed { since: None }],
        "a failure nobody can date is not a fresh one"
    );
}

#[test]
fn a_fresh_failure_holds_the_approval_back_too() {
    let now = Timestamp::new(20 * DAY);
    let t = Thresholds::default();
    let just_now = MrFacts {
        state: Some(MrState::Open),
        approved_at: Some(Timestamp::new(10 * DAY)),
        ci: Some(CiState::Failed),
        ci_finished_at: Some(Timestamp::new(now.seconds() - 60)),
        ..Default::default()
    };
    assert!(
        attention(&just_now, &TaskDates::default(), now, &t).is_empty(),
        "the run is red, so it is not approved and green"
    );
}

#[test]
fn a_thresholds_object_that_names_one_of_them_keeps_the_rest() {
    let read: Thresholds = serde_json::from_str(r#"{"ci_failed_minutes": 30}"#).unwrap();
    assert_eq!(read.ci_failed_minutes, 30);
    assert_eq!(read.review_waiting_days, 3, "the default stands");
    assert_eq!(read.due_soon_days, 2);
}
