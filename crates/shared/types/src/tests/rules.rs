use crate::{
    AgentStatus, ApprovalId, Ask, Attention, AttentionClass, CiState, Day, DiffLine, ExternalId,
    LineKind, MrFacts, MrState, Session, SessionActivity, SessionId, SessionKind, Span, TaskDates,
    Thresholds, Timestamp, attention, names_session, word_diff_pairs,
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
}

fn line(kind: LineKind) -> DiffLine {
    DiffLine {
        num: 0,
        content: String::new(),
        kind,
    }
}

#[test]
fn word_diff_pairs_only_one_for_one_runs() {
    use LineKind::{Add, Ctx, Del};
    let lines: Vec<DiffLine> = [
        Ctx, Del, Del, Add, Add, Ctx, Del, Add, Add, Ctx, Add, Del, Add,
    ]
    .into_iter()
    .map(line)
    .collect();
    assert_eq!(word_diff_pairs(&lines), vec![(1, 3), (2, 4), (11, 12)]);
    assert!(word_diff_pairs(&[line(Add), line(Add)]).is_empty());
    assert!(word_diff_pairs(&[]).is_empty());
}

#[test]
fn dates_draw_a_bar_a_point_or_an_open_bar() {
    let d = |s: &str| Day::parse(s).unwrap();
    let from = d("2026-09-01");
    let with_duration = TaskDates {
        start: Some(from),
        due: Some(d("2026-09-20")),
        duration_days: Some(5),
    };
    assert_eq!(
        with_duration.span(),
        Some(Span::Bar {
            from,
            to: d("2026-09-06")
        })
    );
    let with_due = TaskDates {
        start: Some(from),
        due: Some(d("2026-09-20")),
        duration_days: None,
    };
    assert_eq!(
        with_due.span(),
        Some(Span::Bar {
            from,
            to: d("2026-09-20")
        })
    );
    let only_start = TaskDates {
        start: Some(from),
        ..Default::default()
    };
    assert_eq!(only_start.span(), Some(Span::Open { from }));
    let only_due = TaskDates {
        due: Some(from),
        ..Default::default()
    };
    assert_eq!(only_due.span(), Some(Span::Point(from)));
    assert_eq!(TaskDates::default().span(), None);
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
        auto_approve: false,
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
    });
    assert_eq!(asked.class(), AttentionClass::NeedsYou);
}
