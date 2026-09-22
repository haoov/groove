//! What the row, the header and the overview show of a worktree's merge request.

use groove_gfx::Icon;
use groove_types::{
    CiState, CiStatus, Forge, MrApproval, MrDelivery, MrDetails, MrNote, MrState, MrThread,
    ReviewState, Reviewer, SessionId, Timestamp, WorktreeDelivery,
};

use crate::tests::{app as bare, full_app, metrics};
use crate::{Ui, view};

fn delivery(state: MrState, ci: Option<CiState>) -> WorktreeDelivery {
    WorktreeDelivery {
        mr: Some(MrDelivery {
            forge: Forge::Github,
            number: "7".into(),
            state,
            url: "https://github.com/acme/groove/pull/7".into(),
            approved: false,
            changes_requested: false,
        }),
        ci,
        notes: 2,
        ..Default::default()
    }
}

/// The fixture's worktree carrying this delivery.
fn showing(delivery: WorktreeDelivery) -> groove_controllers::AppState {
    let mut app = full_app();
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session");
    let worktree = open.worktrees[0].id.clone();
    open.delivery.push((worktree, delivery));
    app
}

fn drawn(app: &groove_controllers::AppState) -> (Vec<String>, Vec<Icon>) {
    let (frame, _) = view(
        app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let layer = &frame.layers()[0];
    (
        layer.texts.iter().map(|t| t.text.clone()).collect(),
        layer.icons.iter().map(|i| i.icon).collect(),
    )
}

#[test]
fn an_open_mr_shows_its_number_and_its_notes() {
    let app = showing(delivery(MrState::Open, None));
    let (texts, icons) = drawn(&app);
    assert!(texts.iter().any(|t| t == "#7"), "{texts:?}");
    assert!(icons.contains(&Icon::Chat), "the notes mark");
    assert!(texts.iter().any(|t| t == "2"), "two notes: {texts:?}");
}

#[test]
fn checks_that_failed_show_a_cross_and_ones_that_passed_a_check() {
    let (_, icons) = drawn(&showing(delivery(MrState::Open, Some(CiState::Failed))));
    assert!(icons.contains(&Icon::Cross), "the run failed");

    let (_, icons) = drawn(&showing(delivery(MrState::Open, Some(CiState::Success))));
    assert!(icons.contains(&Icon::Check), "the run passed");

    let (_, icons) = drawn(&showing(delivery(MrState::Open, Some(CiState::Running))));
    assert!(icons.contains(&Icon::Notch), "the run is going");
}

#[test]
fn a_worktree_with_no_mr_draws_nothing_of_one() {
    let (texts, icons) = drawn(&showing(WorktreeDelivery::default()));
    assert!(!texts.iter().any(|t| t.starts_with('#')), "{texts:?}");
    assert!(!icons.contains(&Icon::Chat));
}

#[test]
fn what_the_reviewers_said_shows_beside_the_number() {
    let mut asked = delivery(MrState::Open, None);
    if let Some(mr) = asked.mr.as_mut() {
        mr.changes_requested = true;
    }
    let (_, icons) = drawn(&showing(asked));
    assert!(icons.contains(&Icon::Cross), "changes were requested");

    let mut approved = delivery(MrState::Open, None);
    if let Some(mr) = approved.mr.as_mut() {
        mr.approved = true;
    }
    let (_, icons) = drawn(&showing(approved));
    assert!(icons.contains(&Icon::Check), "it is approved");
}

/// A read of the MR, as the poll leaves it in the slice.
fn read() -> groove_controllers::workspace_service::Snapshot {
    groove_controllers::workspace_service::Snapshot {
        head: "cafe1234".into(),
        node: "PR_node".into(),
        number: "7".into(),
        details: MrDetails {
            title: "fix(forge): read the checks".into(),
            description: String::new(),
            author: "haoov".into(),
            source_branch: "explorer/alpha".into(),
            target_branch: "main".into(),
            state: MrState::Open,
            draft: false,
            created_at: Timestamp::new(0),
            updated_at: Timestamp::new(0),
            web_url: "https://github.com/acme/groove/pull/7".into(),
            approval: Some(MrApproval {
                approved: true,
                approved_by_me: false,
                approved_by: vec!["reviewer".into()],
            }),
            reviewers: vec![Reviewer {
                name: "reviewer".into(),
                state: ReviewState::Approved,
                at: None,
            }],
        },
        ci: Some(CiStatus {
            state: CiState::Failed,
            url: "https://github.com/acme/groove/runs/2".into(),
            finished_at: None,
        }),
        threads: vec![MrThread {
            id: "THREAD_1".into(),
            notes: vec![MrNote {
                author: "reviewer".into(),
                body: "issue: this drops the error".into(),
                created_at: Timestamp::new(0),
                resolved: false,
                resolvable: true,
                position: None,
            }],
        }],
    }
}

#[test]
fn the_overview_names_the_mr_and_what_it_stands_at() {
    let mut app = showing(delivery(MrState::Open, Some(CiState::Failed)));
    app.workspace.worktree = Some(app.session.open[0].worktrees[0].id.clone());
    app.workspace.delivery.read = Some(read());
    app.workspace.delivery.mr = Some(groove_types::Mr {
        id: groove_types::MrId::new("m1"),
        worktree: app.session.open[0].worktrees[0].id.clone(),
        forge: Forge::Github,
        remote_id: "7".into(),
        url: "https://github.com/acme/groove/pull/7".into(),
        state: MrState::Open,
    });
    let (texts, _) = drawn(&app);
    assert!(texts.iter().any(|t| t == "MERGE REQUEST"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "open"), "the state: {texts:?}");
    assert!(texts.iter().any(|t| t == "main"), "the target: {texts:?}");
    assert!(texts.iter().any(|t| t == "failed"), "the checks: {texts:?}");
    assert!(
        texts.iter().any(|t| t == "approved by reviewer"),
        "the review: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "1 of 1 open"),
        "the notes: {texts:?}"
    );
}

#[test]
fn a_session_with_no_mr_read_has_no_merge_request_section() {
    let app = bare();
    let (texts, _) = drawn(&app);
    assert!(!texts.iter().any(|t| t == "MERGE REQUEST"), "{texts:?}");
}

#[test]
fn a_long_branch_is_cut_so_the_row_never_reaches_past_its_area() {
    let mut app = showing(delivery(MrState::Open, Some(CiState::Failed)));
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session");
    open.worktrees[0].branch = "explorer/".to_string() + &"a-very-long-name-".repeat(12);
    let window = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        window,
        &mut groove_gfx::Fonts::embedded(),
    );
    let workspace = crate::layout::Layout::of(window, &Ui::default()).workspace;
    let layer = &frame.layers()[0];
    let over = layer
        .texts
        .iter()
        .filter(|t| t.x >= workspace.right())
        .count();
    assert_eq!(over, 0, "every run starts inside the tab");
    let icons = layer
        .icons
        .iter()
        .filter(|i| i.rect.right() > workspace.right())
        .count();
    assert_eq!(icons, 0, "so does every mark");
}

#[test]
fn the_header_offers_to_read_the_mr_again() {
    let app = showing(delivery(MrState::Open, None));
    let mut ui = Ui::default();
    let window = metrics(1280, 800, 1.0);
    let (_, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let box_ = hits
        .rect_of(&crate::hit::Target::Refresh)
        .expect("the refresh button");
    let commands = crate::input::handle(
        crate::input::Input::Press {
            x: box_.x + box_.w / 2.0,
            y: box_.y + box_.h / 2.0,
        },
        &mut ui,
        &app,
        &hits,
        window,
    );
    assert_eq!(
        commands,
        vec![groove_controllers::Command::Workspace(
            groove_controllers::workspace::Command::RefreshMr
        )]
    );
}
