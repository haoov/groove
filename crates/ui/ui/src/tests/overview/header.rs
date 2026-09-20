//! The session header: its two lines, its pickers, and the task's own actions.

use groove_types::SessionId;

use super::working_a_task;
use crate::tests::{full_app, metrics};
use crate::{Ui, view};

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
fn a_long_title_is_cut_so_the_pickers_stay_inside_the_header() {
    let mut app = full_app();
    app.session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session")
        .session
        .title = "Harden Groove: CI gates, security fixes, defect fixes, typed errors".into();
    let window = metrics(1280, 800, 1.0);
    let ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let header = crate::layout::Layout::of(window, &ui).header;
    for picker in [crate::hit::Picks::Repo, crate::hit::Picks::Branch] {
        let box_ = hits
            .rect_of(&crate::hit::Target::Picker(picker))
            .expect("the header holds both pickers");
        assert!(
            box_.right() <= header.right(),
            "{picker:?} at {box_:?} runs past {header:?}"
        );
    }
    let cut = frame.layers()[0]
        .texts
        .iter()
        .any(|t| t.text.starts_with("Harden Groove") && t.text.ends_with('\u{2026}'));
    assert!(cut, "the title carries the ellipsis");
}

#[test]
fn the_header_holds_the_title_over_the_pickers() {
    let app = full_app();
    let window = metrics(1280, 800, 1.0);
    let ui = Ui::default();
    let (frame, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let layout = crate::layout::Layout::of(window, &ui);
    let tokens = crate::tokens::Tokens::new(1.0);
    let title = frame.layers()[0]
        .texts
        .iter()
        .find(|t| t.text == "Alpha" && t.x >= layout.header.x)
        .expect("the header's title")
        .y;
    let box_ = hits
        .rect_of(&crate::hit::Target::Picker(crate::hit::Picks::Repo))
        .expect("the repo picker");
    assert!(title < tokens.header, "the title is on the first line");
    assert!(box_.y >= tokens.header, "the pickers are on the second");
    assert!(box_.bottom() <= layout.header.bottom());
    assert_eq!(
        layout.workspace.y,
        layout.header.bottom(),
        "the tabs start under both lines"
    );
    let styles = crate::style::Styles::new(app.config.theme(), tokens);
    let grounds = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.band() && quad.rect.h == box_.h)
        .count();
    assert_eq!(grounds, 2, "each picker is a button");
}

#[test]
fn a_task_session_offers_to_finish_and_an_explorer_does_not() {
    let app = working_a_task();
    let (_, hits) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let session = SessionId::new("a");
    assert!(
        hits.rect_of(&crate::hit::Target::Finish(session.clone()))
            .is_some(),
        "the header offers it"
    );

    let plain = full_app();
    let (_, hits) = view(
        &plain,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    assert!(hits.rect_of(&crate::hit::Target::Finish(session)).is_none());
}

#[test]
fn a_worktree_with_an_open_mr_holds_the_finish_back() {
    let mut app = working_a_task();
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session");
    let worktree = open.worktrees[0].id.clone();
    open.delivery.push((
        worktree,
        groove_types::WorktreeDelivery {
            mr: Some(groove_types::MrDelivery {
                state: groove_types::MrState::Open,
                url: "https://example.test/mr/1".into(),
                approved: false,
                changes_requested: false,
            }),
            ..Default::default()
        },
    ));
    let (_, hits) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    assert!(
        hits.rect_of(&crate::hit::Target::Finish(SessionId::new("a")))
            .is_none(),
        "something is still open"
    );
}

#[test]
fn the_task_s_menu_offers_to_delete_the_session_here() {
    let app = working_a_task();
    let mut ui = Ui::default();
    let window = metrics(1280, 800, 1.0);
    let (_, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let session = SessionId::new("a");
    let caret = hits
        .rect_of(&crate::hit::Target::TaskActions(session.clone()))
        .expect("the header offers the rest");
    assert!(
        crate::tests::click(caret, &mut ui, &app, &hits).is_empty(),
        "opening it asks for nothing"
    );
    assert!(ui.menu.is_some(), "the menu stands");

    let (_, hits) = view(&app, &ui, window, &mut groove_gfx::Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::MenuRow(0))
        .expect("it holds a row");
    let commands = crate::tests::click(row, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [groove_controllers::Command::Task(
            groove_controllers::task::Command::DeleteLocal { session }
        )]
    );
}
