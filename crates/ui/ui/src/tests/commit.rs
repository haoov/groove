//! The commit box: its message, its one button, and the menu behind it.

mod forged;
mod typing;

use groove_controllers::{AppState, Command, delivery, workspace};
use groove_gfx::Fonts;
use groove_types::{FileDiff, FileStatus};

use crate::hit::Target;
use crate::input::{Input, handle};
use crate::tests::{drawn_hits, full_app, hit, render, window};
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
    ui.session.tab = Tab::Diff;
    ui
}

#[test]
fn the_box_takes_the_message_once_it_is_clicked() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    assert!(
        drawn_hits(&app, &ui).rect_of(&Target::Message).is_some(),
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
        drawn_hits(&app, &ui).rect_of(&Target::Do).is_none(),
        "there is nothing to commit yet"
    );
    app.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it".into()));
    assert!(
        drawn_hits(&app, &ui).rect_of(&Target::Do).is_some(),
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
    let menu = ui.menu().cloned().expect("the actions are open");
    assert_eq!(
        menu.of,
        crate::Of::Worktree {
            mr: false,
            review: false
        }
    );
    let hits = drawn_hits(&app, &ui);
    for at in 0..crate::views::overlays::actions::WORKTREE.len() {
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
        assert!(ui.losing().is_none(), "and nothing is asked first");
    }
}

#[test]
fn discarding_everything_is_asked_in_the_box_before_it_is_done() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    hit(&Target::Actions, &mut ui, &app);
    let taken = hit(&Target::MenuRow(2), &mut ui, &app);
    assert!(taken.is_empty(), "nothing is thrown away on the spot");
    assert_eq!(ui.losing(), Some(&crate::Losing::Everything));
    assert!(
        drawn_hits(&app, &ui).rect_of(&Target::Do).is_none(),
        "the box is the question now"
    );

    let done = hit(&Target::Discard, &mut ui, &app);
    assert_eq!(done, [Command::Workspace(workspace::Command::DiscardAll)]);
    assert!(ui.losing().is_none());
}

#[test]
fn keeping_everything_asks_nothing_of_git() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.overlay = Some(crate::Overlay::Losing(crate::Losing::Everything));
    let kept = hit(&Target::Keep, &mut ui, &app);
    assert!(kept.is_empty());
    assert!(ui.losing().is_none());
}

/// The one action the box offers now.
fn offered(app: &AppState, ui: &Ui) -> Option<Command> {
    let _ = drawn_hits(app, ui);
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
    let hits = drawn_hits(&app, &ui);
    assert!(hits.rect_of(&Target::Actions).is_some(), "the caret");
    let word = hits.rect_of(&Target::Do);
    assert!(word.is_none(), "with nothing staged there is nothing to do");

    let mut with = listed(&[file("a.txt", true)]);
    with.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it".into()));
    let hits = drawn_hits(&with, &ui);
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
    let hits = drawn_hits(&app, &ui);
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
    let hits = drawn_hits(&app, &ui);
    let box_ = hits.rect_of(&Target::Message).expect("the box");
    let line = groove_ui_kit::base::tokens::Tokens::new(1.0).code_line;
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

#[test]
fn a_commit_box_not_drawn_takes_no_key_and_no_paste() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    ui.settle(&app);
    (ui.focus, ui.session.composing) = (crate::Focus::Sidebar, true);
    for hidden in [Tab::Resources, Tab::Diff] {
        ui.session.tab = hidden;
        ui.session.reading = hidden == Tab::Diff;
        let plain = crate::input::Modifiers::default();
        let typed = crate::tests::press(crate::input::Key::Char('a'), plain, &mut ui, &app);
        let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        let input = crate::input::Input::Paste("b".into());
        let pasted = crate::input::handle(input, &mut ui, &app, &hits, window());
        let message =
            |one: &Command| matches!(one, Command::Workspace(workspace::Command::Message(_)));
        assert!(
            !typed.iter().chain(&pasted).any(message),
            "{hidden:?}: the message takes nothing"
        );
    }
    (ui.focus, ui.session.tab) = (crate::Focus::Workspace, Tab::Resources);
    ui.settle(&app);
    ui.session.tab = Tab::Diff;
    ui.settle(&app);
    assert!(ui.session.commits(), "the box is drawn again");
    assert!(
        !ui.session.composing,
        "but it let go of the keys while hidden"
    );
}
