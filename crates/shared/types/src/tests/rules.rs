use crate::{
    AgentStatus, ApprovalId, Ask, Attention, AttentionClass, CiState, ExternalId, MrFacts, MrState,
    Session, SessionActivity, SessionId, SessionKind, TaskDates, Thresholds, Timestamp, attention,
    names_session,
};

fn task(id: &str) -> Session {
    Session {
        id: SessionId::new(id),
        title: "Fix the diff parser crash".into(),
        kind: SessionKind::Task {
            external_id: ExternalId::new("page"),
        },
        created_at: Timestamp::new(0),
    }
}

#[test]
fn only_this_sessions_own_branch_names_it() {
    let s = task("gh-groove-50");
    assert!(names_session(
        "fix/diff-parser-crash-gh-groove-50",
        &s,
        None
    ));
    assert!(names_session("Fix/GH-GROOVE-50", &s, None));
    assert!(names_session("fix/parser-plat-42", &s, Some("PLAT-42")));
    assert!(!names_session("fix/parser-plat-42", &s, Some("")));
    assert!(!names_session("fix/other-gh-groove-49", &s, None));
    assert!(!names_session("fix/other-gh-groove-500", &s, None));
}

#[test]
fn an_explorer_s_branch_is_its_own_name() {
    let mut s = task("explorer-ab12cd34");
    s.kind = crate::SessionKind::Explorer;
    assert_eq!(crate::explorer_branch(&s.id), "explorer/ab12cd34");
    assert!(names_session("explorer/ab12cd34", &s, None));
    assert!(!names_session("explorer/ff00ff00", &s, None));
}

const DAY: i64 = 86_400;

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

fn activity(status: AgentStatus) -> SessionActivity {
    SessionActivity {
        status,
        tool: None,
        asks: Vec::new(),
        changed_at: Timestamp::new(0),
        seen_at: None,
    }
}

#[test]
fn the_attention_class_follows_asks_then_status() {
    assert_eq!(
        activity(AgentStatus::Working).class(),
        AttentionClass::Moving
    );
    assert_eq!(activity(AgentStatus::Idle).class(), AttentionClass::Quiet);
    assert_eq!(
        activity(AgentStatus::Done { seen: false }).class(),
        AttentionClass::ActWhenYouLook
    );
    assert_eq!(
        activity(AgentStatus::Done { seen: true }).class(),
        AttentionClass::Quiet
    );
    assert_eq!(
        activity(AgentStatus::Exited { code: 1 }).class(),
        AttentionClass::NeedsYou
    );
    let mut asked = activity(AgentStatus::Working);
    asked.asks.push(Ask {
        id: ApprovalId::new("a"),
        op: "git.commit".into(),
        subject: "feat: x".into(),
        text: String::new(),
        worktree: None,
    });
    assert_eq!(asked.class(), AttentionClass::NeedsYou);
}

#[test]
fn the_worst_check_of_a_run_is_the_one_it_reports() {
    use CiState::{Canceled, Failed, Pending, Running, Skipped, Success, Unknown};
    assert_eq!(Success.worst(Failed), Failed);
    assert_eq!(Failed.worst(Running), Failed);
    assert_eq!(Running.worst(Success), Running);
    assert_eq!(Pending.worst(Canceled), Pending, "the run is not done");
    assert_eq!(Skipped.worst(Success), Success, "a run that passed passed");
    assert_eq!(Unknown.worst(Success), Unknown);
}

/// One reviewer's verdict at a time.
fn reviewer(name: &str, state: crate::ReviewState, at: i64) -> crate::Reviewer {
    crate::Reviewer {
        name: name.into(),
        state,
        at: Some(Timestamp::new(at)),
    }
}

fn details(reviewers: Vec<crate::Reviewer>, approved: bool) -> crate::MrDetails {
    crate::MrDetails {
        title: String::new(),
        description: String::new(),
        author: "haoov".into(),
        source_branch: "one".into(),
        target_branch: "main".into(),
        state: MrState::Open,
        draft: false,
        created_at: Timestamp::new(0),
        updated_at: Timestamp::new(0),
        web_url: String::new(),
        approval: Some(crate::MrApproval {
            approved,
            approved_by_me: false,
            approved_by: Vec::new(),
        }),
        reviewers,
    }
}

#[test]
fn the_times_the_rules_rest_on_come_off_the_reviewers() {
    use crate::ReviewState::{Approved, ChangesRequested, Requested};
    let mr = details(
        vec![
            reviewer("one", Requested, 5 * DAY),
            reviewer("two", Requested, 3 * DAY),
            reviewer("three", ChangesRequested, 7 * DAY),
            reviewer("four", ChangesRequested, 9 * DAY),
        ],
        false,
    );
    assert_eq!(
        mr.review_requested_at(),
        Some(Timestamp::new(3 * DAY)),
        "waiting since the first was asked"
    );
    assert_eq!(
        mr.changes_requested_at(),
        Some(Timestamp::new(7 * DAY)),
        "and asked for changes since the first said so"
    );
    assert!(mr.changes_requested());
    assert_eq!(mr.approved_at(), None);

    let approved = details(
        vec![
            reviewer("one", Approved, 4 * DAY),
            reviewer("two", Approved, 8 * DAY),
        ],
        true,
    );
    assert_eq!(
        approved.approved_at(),
        Some(Timestamp::new(8 * DAY)),
        "approved when the last one approved"
    );
    let short = details(vec![reviewer("one", Approved, 4 * DAY)], false);
    assert_eq!(short.approved_at(), None, "one approval was not enough");
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

#[test]
fn a_search_ignores_case_and_answers_in_characters_of_the_line_itself() {
    use crate::occurrences;
    assert_eq!(occurrences("Fix fix FIX", "fix"), [0..3, 4..7, 8..11]);
    assert_eq!(occurrences("İstanbul é Straße", "STRASSE"), []);
    assert_eq!(occurrences("İstanbul é Straße", "straße"), vec![11..17]);
    assert_eq!(occurrences("aaaa", "aa"), [0..2, 2..4]);
    assert!(occurrences("text", "").is_empty());
}
