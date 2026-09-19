//! The commit box: its message, its one button, and the menu behind it.

use groove_controllers::{AppState, Command, workspace};
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
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace
        .loaded(worktree, files.to_vec(), Default::default());
    app
}

fn sidebar() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
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
    assert_eq!(menu.of, crate::Of::Worktree);
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
fn offered(app: &AppState, ui: &Ui) -> Option<workspace::Command> {
    let _ = drawn(app, ui);
    crate::views::session::commit::primary(app)
}

#[test]
fn the_box_offers_what_the_worktree_most_wants_doing() {
    let mut app = listed(&[file("a.txt", false)]);
    let ui = sidebar();
    assert_eq!(
        offered(&app, &ui),
        Some(workspace::Command::Commit),
        "a change wants committing"
    );

    app.workspace.loaded(
        app.session
            .selected()
            .and_then(|open| open.selected_worktree())
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
