//! The session header and the overview tab: what they name and what they count.

use groove_types::SessionId;

use crate::tests::{app, full_app, metrics};
use crate::{Ui, view};

#[test]
fn the_overview_lists_repos_and_worktrees_with_the_selected_one_marked() {
    let app = full_app();
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "overview"));
    assert!(texts.iter().any(|t| t == "REPOS AND WORKTREES"));
    assert!(texts.iter().any(|t| t == "mayo"));
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the header and the row"
    );
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(
        icons.contains(&groove_gfx::Icon::Cube),
        "the repo wears a mark"
    );
    assert!(
        icons.contains(&groove_gfx::Icon::Compass),
        "the session's kind"
    );
}

#[test]
fn a_worktrees_counts_show_as_icons_and_zeros_do_not() {
    let mut app = full_app();
    let open = app.session.get_mut(&SessionId::new("a")).unwrap();
    let worktree = open.worktrees[0].id.clone();
    open.delivery.push((
        worktree,
        groove_types::WorktreeDelivery {
            status: groove_types::WorktreeStatus {
                modified: 3,
                staged: 0,
                ahead: 1,
                behind: 0,
            },
            ..Default::default()
        },
    ));
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let icons: Vec<groove_gfx::Icon> = frame.layers()[0].icons.iter().map(|i| i.icon).collect();
    assert!(icons.contains(&groove_gfx::Icon::ArrowUp), "ahead 1");
    assert!(icons.contains(&groove_gfx::Icon::Dot), "modified 3");
    assert!(
        !icons.contains(&groove_gfx::Icon::Plus),
        "staged 0 is not drawn"
    );
    assert!(
        !icons.contains(&groove_gfx::Icon::ArrowDown),
        "behind 0 is not drawn"
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "3"));
    assert!(texts.iter().any(|t| t == "1"));
}

#[test]
fn the_header_names_the_session_and_what_it_points_at() {
    let app = full_app();
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "Alpha"), "the title");
    assert!(texts.iter().any(|t| t == "mayo"), "the repo picker");
    assert_eq!(
        texts.iter().filter(|t| *t == "explorer/alpha").count(),
        2,
        "the worktree picker and the overview row"
    );
    let carets = frame.layers()[0]
        .icons
        .iter()
        .filter(|i| i.icon == groove_gfx::Icon::CaretDown)
        .count();
    assert_eq!(carets, 2, "one per picker");
}

#[test]
fn a_session_without_a_worktree_says_so() {
    let mut app = app();
    app.session.selected = Some(SessionId::new("a"));
    let metrics = metrics(1280, 800, 1.0);
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics,
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
    assert!(texts.iter().any(|t| t == "no repo"));
    assert!(texts.iter().any(|t| t == "no worktree"));
}
