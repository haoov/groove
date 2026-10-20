//! The merge request and review actions the box offers.

use super::*;

/// The worktree pushed and clean, its forge already asked about.
fn landed() -> AppState {
    let mut app = listed(&[]);
    let worktree = app
        .session
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.delivery.poll.sent(&worktree);
    app.delivery.poll.answered(&worktree);
    app
}

#[test]
fn a_branch_with_nothing_left_to_push_offers_a_merge_request() {
    let app = landed();
    assert_eq!(
        crate::views::session::commit::primary(&app),
        Some(Command::Delivery(delivery::Command::CreateMr))
    );
    let (texts, _) = render(&app, &sidebar(), window());
    assert!(texts.iter().any(|t| t == "open mr"), "{texts:?}");
}

#[test]
fn a_branch_whose_forge_has_not_answered_offers_nothing() {
    let app = listed(&[]);
    assert_eq!(
        crate::views::session::commit::primary(&app),
        None,
        "nothing is known of its mr yet"
    );
}

#[test]
fn a_branch_that_already_has_one_offers_nothing_on_the_button() {
    let mut app = landed();
    app.delivery.remembered(vec![groove_types::Mr {
        id: groove_types::MrId::new("m1"),
        worktree: groove_types::WorktreeId::new("wt-1"),
        forge: groove_types::Forge::Github,
        remote_id: "7".into(),
        url: "https://example.test/pull/7".into(),
        state: groove_types::MrState::Open,
    }]);
    assert_eq!(crate::views::session::commit::primary(&app), None);
}

#[test]
fn the_menu_offers_the_mr_writes_only_where_there_is_one_to_write() {
    let plain = crate::views::overlays::actions::rows(&crate::Of::Worktree {
        mr: false,
        review: false,
    });
    assert!(!plain.contains(&"update mr"), "{plain:?}");

    let with_mr = crate::views::overlays::actions::rows(&crate::Of::Worktree {
        mr: true,
        review: false,
    });
    assert!(with_mr.contains(&"update mr"), "{with_mr:?}");
    assert!(with_mr.contains(&"close mr"), "{with_mr:?}");
    let at = with_mr
        .iter()
        .position(|row| *row == "close mr")
        .expect("the row");
    let picked = crate::views::overlays::actions::picked(
        &crate::Of::Worktree {
            mr: true,
            review: false,
        },
        at,
    );
    let (commands, asking) = (picked.commands, picked.asking);
    assert!(asking.is_none(), "closing an mr asks nothing first");
    assert_eq!(
        commands,
        vec![Command::Delivery(delivery::Command::CloseMr)]
    );
}

#[test]
fn a_review_session_offers_a_verdict_and_a_comment() {
    let rows = crate::views::overlays::actions::rows(&crate::Of::Worktree {
        mr: true,
        review: true,
    });
    for named in ["comment", "approve", "request changes"] {
        assert!(rows.contains(&named), "{named}: {rows:?}");
    }
    assert!(
        !rows.contains(&"update mr"),
        "someone else's text is not ours to write: {rows:?}"
    );
}

#[test]
fn a_session_of_its_own_work_offers_a_comment_and_no_verdict() {
    let rows = crate::views::overlays::actions::rows(&crate::Of::Worktree {
        mr: true,
        review: false,
    });
    assert!(rows.contains(&"comment"), "{rows:?}");
    for named in ["approve", "request changes"] {
        assert!(!rows.contains(&named), "{named}: {rows:?}");
    }
}

#[test]
fn picking_a_verdict_asks_for_the_review() {
    use groove_controllers::delivery::Say;
    let of = crate::Of::Worktree {
        mr: true,
        review: true,
    };
    let rows = crate::views::overlays::actions::rows(&of);
    let at = |label: &str| rows.iter().position(|one| *one == label).expect(label);
    let picked = |label: &str| crate::views::overlays::actions::picked(&of, at(label)).commands;
    assert_eq!(
        picked("approve"),
        [groove_controllers::Command::Delivery(
            delivery::Command::Say(Say::Review(groove_types::ReviewVerdict::Approve))
        )]
    );
    assert_eq!(
        picked("request changes"),
        [groove_controllers::Command::Delivery(
            delivery::Command::Say(Say::Review(groove_types::ReviewVerdict::RequestChanges))
        )]
    );
    assert_eq!(
        picked("comment"),
        [groove_controllers::Command::Delivery(
            delivery::Command::Say(Say::Review(groove_types::ReviewVerdict::Comment))
        )]
    );
}
