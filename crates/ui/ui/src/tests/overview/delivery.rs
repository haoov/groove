//! What the row, the header and the overview show of a worktree's merge request.

use groove_controllers::AppState;
use groove_controllers::delivery_service::{Delivered, Snapshot};
use groove_gfx::Icon;
use groove_types::{
    CiState, CiStatus, Forge, Mr, MrApproval, MrDetails, MrId, MrNote, MrState, MrThread,
    ReviewState, Reviewer, SessionId, Timestamp, WorktreeId,
};

use crate::tests::{full_app, metrics};
use crate::{Ui, view};
use groove_ui_kit::base::style::{Role, Styles};
use groove_ui_kit::base::tokens::Tokens;

/// The MR row the database holds, in this state.
fn mr(worktree: &WorktreeId, state: MrState) -> Mr {
    Mr {
        id: MrId::new("m1"),
        worktree: worktree.clone(),
        forge: Forge::Github,
        remote_id: "7".into(),
        url: "https://github.com/acme/groove/pull/7".into(),
        state,
    }
}

/// A read of the MR in this state: its run, no reviewer yet, two threads nobody resolved.
fn snapshot(state: MrState, ci: Option<CiState>) -> Snapshot {
    let mut read = read();
    read.details.state = state;
    read.details.approval = None;
    read.details.reviewers = Vec::new();
    read.ci = ci.map(|state| CiStatus {
        state,
        url: String::new(),
        finished_at: None,
    });
    read.threads = vec![thread("t1"), thread("t2")];
    read
}

fn thread(id: &str) -> MrThread {
    MrThread {
        id: id.into(),
        notes: vec![MrNote {
            author: "reviewer".into(),
            body: "issue: this drops the error".into(),
            created_at: Timestamp::new(0),
            resolved: false,
            resolvable: true,
            position: None,
        }],
    }
}

/// The fixture's worktree, its MR read as the poll leaves it.
fn showing(state: MrState, read: Snapshot) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .get(&SessionId::new("a"))
        .expect("the fixture's session")
        .worktrees[0]
        .id
        .clone();
    let mr = mr(&worktree, state);
    let now = groove_types::Timestamp::now();
    app.delivery.took(
        &worktree,
        Delivered {
            mr,
            read,
            unassigned: None,
        },
        now,
    );
    app
}

fn open_with(ci: Option<CiState>) -> AppState {
    showing(MrState::Open, snapshot(MrState::Open, ci))
}

fn drawn(app: &AppState) -> (Vec<String>, Vec<Icon>) {
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
    let app = open_with(None);
    let (texts, icons) = drawn(&app);
    assert!(texts.iter().any(|t| t == "#7"), "{texts:?}");
    assert!(icons.contains(&Icon::Chat), "the notes mark");
    assert!(texts.iter().any(|t| t == "2"), "two notes: {texts:?}");
}

/// The colour `word` is drawn in, where it is drawn.
fn colour_of(app: &AppState, word: &str) -> Option<groove_gfx::Color> {
    let (frame, _) = view(
        app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts = &frame.layers()[0].texts;
    texts.iter().find(|t| t.text == word).map(|t| t.style.color)
}

fn role(app: &AppState, role: Role) -> Option<groove_gfx::Color> {
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    Some(styles.color(role))
}

#[test]
fn ci_is_a_word_in_the_colour_of_its_run() {
    for (ci, wanted) in [
        (CiState::Failed, Role::Bad),
        (CiState::Success, Role::Ok),
        (CiState::Running, Role::Attention),
        (CiState::Canceled, Role::Ghost),
    ] {
        let app = open_with(Some(ci));
        assert_eq!(colour_of(&app, "CI"), role(&app, wanted), "{ci:?}");
    }
}

#[test]
fn ci_stands_on_a_rounded_ground_and_edge_of_its_colour() {
    use groove_ui_kit::base::tokens::{BADGE_EDGE, BADGE_GROUND};
    let app = open_with(Some(CiState::Failed));
    let bad = role(&app, Role::Bad).expect("a colour");
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let layer = &frame.layers()[0];
    let word = layer.texts.iter().find(|t| t.text == "CI").expect("CI");
    let under = |alpha: u8, stroked: bool| {
        layer.quads.iter().any(|q| {
            q.color == bad.with_alpha(alpha)
                && q.shape.radius > 0.0
                && (q.shape.stroke > 0.0) == stroked
                && q.rect.x <= word.x
                && word.x <= q.rect.right()
        })
    };
    assert!(under(BADGE_GROUND, false), "the ground");
    assert!(under(BADGE_EDGE, true), "the edge");
}

#[test]
fn a_merged_mr_says_merged_in_purple_in_place_of_its_review_and_its_run() {
    let app = showing(
        MrState::Merged,
        snapshot(MrState::Merged, Some(CiState::Success)),
    );
    assert_eq!(colour_of(&app, "merged"), role(&app, Role::Merged));
    assert!(colour_of(&app, "CI").is_none());
}

#[test]
fn a_worktree_with_no_mr_draws_nothing_of_one() {
    let (texts, icons) = drawn(&full_app());
    assert!(!texts.iter().any(|t| t.starts_with('#')), "{texts:?}");
    assert!(!icons.contains(&Icon::Chat));
}

/// The fixture's MR with one reviewer standing at `state`, approved when it says so.
fn reviewed(state: ReviewState, approved: bool) -> AppState {
    let mut read = snapshot(MrState::Open, None);
    read.details.reviewers = vec![Reviewer {
        name: "reviewer".into(),
        state,
        at: None,
    }];
    read.details.approval = Some(MrApproval {
        approved,
        approved_by_me: false,
        approved_by: Vec::new(),
    });
    showing(MrState::Open, read)
}

#[test]
fn what_the_reviewers_said_is_a_word_beside_the_number() {
    for (state, approved, word, wanted) in [
        (ReviewState::Approved, true, "approved", Role::Ok),
        (ReviewState::Commented, false, "comments", Role::Attention),
        (ReviewState::ChangesRequested, false, "changes", Role::Bad),
        (ReviewState::Requested, false, "review", Role::Attention),
    ] {
        let app = reviewed(state, approved);
        assert_eq!(colour_of(&app, word), role(&app, wanted), "{state:?}");
    }
}

#[test]
fn comments_left_outweigh_an_approval() {
    let app = reviewed(ReviewState::Commented, true);
    assert!(colour_of(&app, "comments").is_some());
    assert!(colour_of(&app, "approved").is_none());
}

/// A read of the MR, as the poll leaves it in the slice.
fn read() -> Snapshot {
    Snapshot {
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
fn a_comment_nobody_can_resolve_is_not_an_open_note() {
    let mut read = snapshot(MrState::Open, None);
    read.threads[1].notes[0].resolvable = false;
    let (texts, _) = drawn(&showing(MrState::Open, read));
    assert!(texts.iter().any(|t| t == "1"), "one open note: {texts:?}");
    assert!(!texts.iter().any(|t| t == "2"), "{texts:?}");
}

#[test]
fn the_mr_stands_after_the_branch_picker_and_opens_its_page() {
    let app = open_with(None);
    let mut ui = Ui::default();
    let window = metrics(1280, 800, 1.0);
    let (_, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let picker = crate::hit::Target::Picker(crate::hit::Picks::Branch);
    let branch = hits.rect_of(&picker).expect("the branch picker");
    let url = "https://github.com/acme/groove/pull/7".to_string();
    let link = hits
        .rect_of(&crate::hit::Target::MrPage(url.clone()))
        .expect("the MR's number");
    assert!(link.x - branch.right() < 16.0, "{link:?} after {branch:?}");

    let commands = crate::input::handle(
        crate::input::Input::Press {
            x: link.x + link.w / 2.0,
            y: link.y + link.h / 2.0,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window,
    );
    let browse = groove_controllers::delivery::Command::BrowseMr { url };
    assert_eq!(
        commands,
        vec![groove_controllers::Command::Delivery(browse)]
    );
}

#[test]
fn a_long_branch_is_cut_so_the_row_never_reaches_past_its_area() {
    let mut app = open_with(Some(CiState::Failed));
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
    let app = open_with(None);
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
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window,
    );
    assert_eq!(
        commands,
        vec![groove_controllers::Command::Delivery(
            groove_controllers::delivery::Command::RefreshMr
        )]
    );
}

#[test]
fn a_review_s_overview_reads_the_mr_s_description() {
    let mut read = snapshot(MrState::Open, None);
    read.details.description = "Reads the checks off the head commit.".into();
    let mut app = showing(MrState::Open, read);
    if let Some(open) = app.session.get_mut(&SessionId::new("a")) {
        open.session.kind = groove_types::SessionKind::Review {
            project: "acme/groove".into(),
            iid: 7,
        };
    }
    let (texts, _) = drawn(&app);
    assert!(texts.iter().any(|t| t == "DESCRIPTION"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t == "Reads the checks off the head commit."),
        "{texts:?}"
    );
}
