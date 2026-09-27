//! The commit box: its message, its one button, and the menu behind it.

use groove_controllers::{AppState, Command, delivery, workspace};
use groove_gfx::Fonts;
use groove_types::{FileDiff, FileStatus};

use crate::hit::Target;
use crate::input::{Input, handle};
use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Ui, view};

fn file(path: &str, staged: bool) -> FileDiff {
    FileDiff {
        path: path.into(),
        added: 2,
        deleted: 1,
        status: FileStatus::Modified,
        staged: Some(staged),
    }
}

fn listed(files: &[FileDiff]) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace
        .loaded(worktree, files.to_vec(), Default::default());
    app
}

fn sidebar() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui
}

/// The targets the sidebar drew, with the pointer wherever `hover` says.
fn drawn(app: &AppState, ui: &Ui) -> crate::hit::Hits {
    view(app, ui, window(), &mut Fonts::embedded()).1
}

/// A click in the middle of a target the last frame drew.
fn hit(target: &Target, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let hits = drawn(app, ui);
    let rect = hits
        .rect_of(target)
        .unwrap_or_else(|| panic!("{target:?} was drawn"));
    handle(
        Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        ui,
        app,
        &hits,
        window(),
    )
}

#[test]
fn the_box_takes_the_message_once_it_is_clicked() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    assert!(
        drawn(&app, &ui).rect_of(&Target::Message).is_some(),
        "the box is there before it is written in"
    );
    hit(&Target::Message, &mut ui, &app);
    assert!(ui.session.composing, "and it takes the keyboard on a click");

    let typed = crate::tests::press(
        crate::input::Key::Char('f'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert_eq!(
        typed,
        [Command::Workspace(workspace::Command::Message(
            groove_types::Edit::Insert("f".into())
        ))],
        "a letter goes to the message, not the list"
    );
}

#[test]
fn a_line_of_the_message_is_a_line_not_a_commit() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    ui.focus = crate::Focus::Sidebar;
    ui.session.composing = true;
    let plain = crate::input::Modifiers::default();
    let entered = crate::tests::press(crate::input::Key::Enter, plain, &mut ui, &app);
    assert_eq!(
        entered,
        [Command::Workspace(workspace::Command::Message(
            groove_types::Edit::Newline
        ))]
    );
    let with_ctrl = crate::input::Modifiers {
        ctrl: true,
        ..plain
    };
    let committed = crate::tests::press(crate::input::Key::Enter, with_ctrl, &mut ui, &app);
    assert_eq!(
        committed,
        [Command::Workspace(workspace::Command::Commit)],
        "control and enter commits"
    );
    assert!(!ui.session.composing, "and the box lets the keyboard go");
}

#[test]
fn nothing_commits_an_empty_message() {
    let mut app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    assert!(
        drawn(&app, &ui).rect_of(&Target::Do).is_none(),
        "there is nothing to commit yet"
    );
    app.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it".into()));
    assert!(
        drawn(&app, &ui).rect_of(&Target::Do).is_some(),
        "and now there is"
    );
    let asked = hit(&Target::Do, &mut ui, &app);
    assert_eq!(asked, [Command::Workspace(workspace::Command::Commit)]);
}

#[test]
fn escape_gives_the_keyboard_back_to_the_list() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    ui.focus = crate::Focus::Sidebar;
    ui.session.composing = true;
    let left = crate::tests::press(
        crate::input::Key::Escape,
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert!(left.is_empty());
    assert!(!ui.session.composing);
}

#[test]
fn the_box_offers_the_worktree_s_own_actions() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    hit(&Target::Actions, &mut ui, &app);
    let menu = ui.menu.clone().expect("the actions are open");
    assert_eq!(
        menu.of,
        crate::Of::Worktree {
            mr: false,
            review: false
        }
    );
    let hits = drawn(&app, &ui);
    for at in 0..crate::views::shared::actions::WORKTREE.len() {
        assert!(
            hits.rect_of(&Target::MenuRow(at)).is_some(),
            "row {at} is drawn"
        );
    }
}

#[test]
fn push_and_pull_are_asked_for_on_the_spot() {
    let app = listed(&[file("a.txt", false)]);
    for (at, wanted) in [(0, workspace::Command::Push), (1, workspace::Command::Pull)] {
        let mut ui = sidebar();
        hit(&Target::Actions, &mut ui, &app);
        let asked = hit(&Target::MenuRow(at), &mut ui, &app);
        assert_eq!(asked, [Command::Workspace(wanted)], "row {at}");
        assert!(ui.discarding.is_none(), "and nothing is asked first");
    }
}

#[test]
fn discarding_everything_is_asked_in_the_box_before_it_is_done() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    hit(&Target::Actions, &mut ui, &app);
    let taken = hit(&Target::MenuRow(2), &mut ui, &app);
    assert!(taken.is_empty(), "nothing is thrown away on the spot");
    assert_eq!(ui.discarding, Some(crate::Losing::Everything));
    assert!(
        drawn(&app, &ui).rect_of(&Target::Do).is_none(),
        "the box is the question now"
    );

    let done = hit(&Target::Discard, &mut ui, &app);
    assert_eq!(done, [Command::Workspace(workspace::Command::DiscardAll)]);
    assert!(ui.discarding.is_none());
}

#[test]
fn keeping_everything_asks_nothing_of_git() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.discarding = Some(crate::Losing::Everything);
    let kept = hit(&Target::Keep, &mut ui, &app);
    assert!(kept.is_empty());
    assert!(ui.discarding.is_none());
}

/// The one action the box offers now.
fn offered(app: &AppState, ui: &Ui) -> Option<Command> {
    let _ = drawn(app, ui);
    crate::views::session::commit::primary(app)
}

#[test]
fn the_box_offers_what_the_worktree_most_wants_doing() {
    let mut app = listed(&[file("a.txt", false)]);
    let ui = sidebar();
    assert_eq!(
        offered(&app, &ui),
        Some(Command::Workspace(workspace::Command::Commit)),
        "a change wants committing"
    );

    app.workspace.loaded(
        app.session
            .selected_worktree()
            .map(|w| w.id.clone())
            .expect("a worktree"),
        Vec::new(),
        Default::default(),
    );
    assert_eq!(offered(&app, &ui), None, "a clean worktree wants nothing");
}

#[test]
fn only_one_action_is_a_button_and_the_caret_holds_the_rest() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let hits = drawn(&app, &ui);
    assert!(hits.rect_of(&Target::Actions).is_some(), "the caret");
    let word = hits.rect_of(&Target::Do);
    assert!(word.is_none(), "with nothing staged there is nothing to do");

    let mut with = listed(&[file("a.txt", true)]);
    with.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it".into()));
    let hits = drawn(&with, &ui);
    assert!(hits.rect_of(&Target::Do).is_some(), "the word acts");
    assert!(
        hits.rect_of(&Target::Actions).is_some(),
        "the caret is beside it"
    );
    let asked = hit(&Target::Do, &mut ui, &with);
    assert_eq!(asked, [Command::Workspace(workspace::Command::Commit)]);
}

#[test]
fn a_click_in_the_message_puts_the_caret_where_it_landed() {
    let mut app = listed(&[file("a.txt", true)]);
    app.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it\nthe body".into()));
    let mut ui = sidebar();
    let hits = drawn(&app, &ui);
    let box_ = hits.rect_of(&Target::Message).expect("the box");
    let cell = window().cell;
    let point = (box_.x + cell.width * 4.0, box_.y + cell.height / 2.0);
    let commands = handle(
        Input::Press {
            x: point.0,
            y: point.1,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Message(
            groove_types::Edit::Move(groove_types::Motion::To(groove_types::Caret::new(0, 4)))
        ))],
        "the caret goes to the fourth character of the first line"
    );
}

#[test]
fn a_click_on_the_second_line_of_the_message_lands_on_it() {
    let mut app = listed(&[file("a.txt", true)]);
    app.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it\nthe body".into()));
    let mut ui = sidebar();
    let hits = drawn(&app, &ui);
    let box_ = hits.rect_of(&Target::Message).expect("the box");
    let line = crate::tokens::Tokens::new(1.0).line;
    let x = box_.x + window().cell.width * 2.0;
    let commands = handle(
        Input::Press {
            x,
            y: box_.y + line * 1.5,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Message(
            groove_types::Edit::Move(groove_types::Motion::To(groove_types::Caret::new(1, 2)))
        ))]
    );
}

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
    let (frame, _) = view(&app, &sidebar(), window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|t| t.text.clone())
        .collect();
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
    let plain = crate::views::shared::actions::rows(&crate::Of::Worktree {
        mr: false,
        review: false,
    });
    assert!(!plain.contains(&"update mr"), "{plain:?}");

    let with_mr = crate::views::shared::actions::rows(&crate::Of::Worktree {
        mr: true,
        review: false,
    });
    assert!(with_mr.contains(&"update mr"), "{with_mr:?}");
    assert!(with_mr.contains(&"close mr"), "{with_mr:?}");
    let at = with_mr
        .iter()
        .position(|row| *row == "close mr")
        .expect("the row");
    let picked = crate::views::shared::actions::picked(
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
    let rows = crate::views::shared::actions::rows(&crate::Of::Worktree {
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
    let rows = crate::views::shared::actions::rows(&crate::Of::Worktree {
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
    let rows = crate::views::shared::actions::rows(&of);
    let at = |label: &str| rows.iter().position(|one| *one == label).expect(label);
    let picked = |label: &str| crate::views::shared::actions::picked(&of, at(label)).commands;
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
            delivery::Command::Say(Say::Comment)
        )]
    );
}
