//! What a file's row offers: the index, and the question before a change is lost.

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
    app.workspace.loaded(worktree, files.to_vec());
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
fn a_row_offers_the_index_only_while_the_pointer_is_on_it() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let stage = Target::Stage("a.txt".into());
    assert!(
        drawn(&app, &ui).rect_of(&stage).is_none(),
        "nothing is offered to a pointer that is elsewhere"
    );
    ui.hover = Some(Target::File("a.txt".into()));
    assert!(
        drawn(&app, &ui).rect_of(&stage).is_some(),
        "and stage appears on the row the pointer is on"
    );
}

#[test]
fn a_staged_row_offers_the_way_back_out() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::File("a.txt".into()));
    let hits = drawn(&app, &ui);
    assert!(hits.rect_of(&Target::Unstage("a.txt".into())).is_some());
    assert!(hits.rect_of(&Target::Stage("a.txt".into())).is_none());
}

#[test]
fn the_offer_stays_while_the_pointer_is_on_the_offer_itself() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::Stage("a.txt".into()));
    assert!(
        drawn(&app, &ui)
            .rect_of(&Target::Stage("a.txt".into()))
            .is_some(),
        "or it would go as the pointer reached it"
    );
}

#[test]
fn clicking_the_offer_asks_for_the_index() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::File("a.txt".into()));
    let asked = hit(&Target::Stage("a.txt".into()), &mut ui, &app);
    assert_eq!(
        asked,
        [Command::Workspace(workspace::Command::Stage {
            path: "a.txt".into()
        })]
    );
}

#[test]
fn the_right_button_opens_a_row_s_actions_and_a_click_closes_them() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let hits = drawn(&app, &ui);
    let row = hits
        .rect_of(&Target::File("a.txt".into()))
        .expect("a row is drawn");
    let point = (row.x + row.w / 2.0, row.y + row.h / 2.0);
    handle(
        Input::Menu {
            x: point.0,
            y: point.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let menu = ui.menu.clone().expect("the actions are open");
    assert_eq!(menu.of, crate::Of::File("a.txt".into()));
    assert!(
        drawn(&app, &ui).rect_of(&Target::MenuRow(0)).is_some(),
        "and drawn"
    );

    let taken = hit(&Target::MenuRow(0), &mut ui, &app);
    assert!(taken.is_empty(), "discard is not done on the spot");
    assert!(ui.menu.is_none(), "the actions close");
    assert_eq!(
        ui.discarding,
        Some(crate::Losing::File("a.txt".into())),
        "the row is asking instead"
    );
}

#[test]
fn the_row_asks_before_a_change_is_thrown_away() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.discarding = Some(crate::Losing::File("a.txt".into()));
    let hits = drawn(&app, &ui);
    assert!(
        hits.rect_of(&Target::File("a.txt".into())).is_none(),
        "the row is the question now, not the file"
    );
    assert!(hits.rect_of(&Target::Keep).is_some());

    let done = hit(&Target::Discard, &mut ui, &app);
    assert_eq!(
        done,
        [Command::Workspace(workspace::Command::Discard {
            path: "a.txt".into()
        })]
    );
    assert!(ui.discarding.is_none(), "and the question is answered");
}

#[test]
fn keeping_the_change_asks_nothing_of_git() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.discarding = Some(crate::Losing::File("a.txt".into()));
    let kept = hit(&Target::Keep, &mut ui, &app);
    assert!(kept.is_empty(), "nothing is asked");
    assert!(ui.discarding.is_none());
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
fn the_menu_stands_above_the_rows_it_covers() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let hits = drawn(&app, &ui);
    let row = hits
        .rect_of(&Target::File("a.txt".into()))
        .expect("a row is drawn");
    handle(
        Input::Menu {
            x: row.x + row.w / 2.0,
            y: row.y + row.h / 2.0,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        frame.layers().len() > 1,
        "a layer of its own, or the text under it draws over it"
    );
    let above = frame.layers().last().expect("the top layer");
    assert!(
        !above.texts.is_empty() && !above.quads.is_empty(),
        "and the menu is what is on it"
    );
}

#[test]
fn a_row_under_the_pointer_and_the_open_one_read_apart() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
    let row = crate::tokens::Tokens::new(1.0).row;
    let grounds = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        let quads = frame.layers()[0].quads.clone();
        let of = |color| {
            quads
                .iter()
                .filter(|quad| quad.color == color && quad.rect.h == row)
                .count()
        };
        (of(styles.hover()), of(styles.here()))
    };
    assert_eq!(grounds(&ui), (0, 0), "a row at rest carries neither");
    ui.hover = Some(Target::File("a.txt".into()));
    assert_eq!(grounds(&ui).0, 1, "the pointer grounds the row");
    assert_ne!(styles.hover(), styles.here(), "and the two never match");
    assert_ne!(
        styles.hover(),
        styles.action(),
        "nor does a button's ground"
    );
}

#[test]
fn the_offer_takes_a_ground_only_once_the_pointer_is_on_it() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
    let lit = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.action())
            .count()
    };
    ui.hover = Some(Target::File("a.txt".into()));
    assert_eq!(lit(&ui), 0, "the word is there, with no ground of its own");
    ui.hover = Some(Target::Stage("a.txt".into()));
    assert_eq!(lit(&ui), 1, "and it lights up under the pointer");
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
fn push_pull_and_rebase_are_asked_for_on_the_spot() {
    let app = listed(&[file("a.txt", false)]);
    for (at, wanted) in [
        (0, workspace::Command::Push),
        (1, workspace::Command::Pull),
        (2, workspace::Command::Rebase),
    ] {
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
    let taken = hit(&Target::MenuRow(3), &mut ui, &app);
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
    crate::views::session::components::commit::primary(app)
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
    let word = hits.rect_of(&Target::Do).expect("the word acts");
    let caret = hits.rect_of(&Target::Actions).expect("the caret");
    assert!(word.right() <= caret.x + 1.0, "the caret is at its end");
    let asked = hit(&Target::Do, &mut ui, &with);
    assert_eq!(asked, [Command::Workspace(workspace::Command::Commit)]);
}

#[test]
fn the_worktree_s_menu_stands_above_the_caret_that_opened_it() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    hit(&Target::Actions, &mut ui, &app);
    let menu = ui.menu.clone().expect("the actions are open");
    assert_eq!(menu.corner, crate::Corner::BottomRight, "it hangs upwards");
    let hits = drawn(&app, &ui);
    let box_ = crate::layout::Layout::of(window(), &ui).commit;
    let caret = hits.rect_of(&Target::Actions).expect("the caret");
    let first = hits.rect_of(&Target::MenuRow(0)).expect("a row");
    let last = hits
        .rect_of(&Target::MenuRow(
            crate::views::shared::actions::WORKTREE.len() - 1,
        ))
        .expect("the last row");
    assert!(
        last.bottom() <= box_.y + 1.0,
        "it ends at the box's own edge"
    );
    assert!(
        (first.right() - caret.right()).abs() <= 1.0,
        "and its own right edge is the caret's"
    );
}

#[test]
fn a_menu_row_is_as_wide_as_what_it_says() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    hit(&Target::Actions, &mut ui, &app);
    let hits = drawn(&app, &ui);
    let row = hits
        .rect_of(&Target::MenuRow(3))
        .expect("discard every change");
    let mut fonts = Fonts::embedded();
    let (frame, _) = view(&app, &ui, window(), &mut fonts);
    let text = frame
        .layers()
        .last()
        .expect("the menu's layer")
        .texts
        .iter()
        .find(|run| run.text.contains("discard"))
        .expect("the longest row");
    let pad = crate::tokens::Tokens::new(1.0).md;
    assert!(
        text.x + pad <= row.right(),
        "its text fits with room at both ends"
    );
}

#[test]
fn the_caret_sits_in_the_middle_of_its_own_room() {
    let mut app = listed(&[file("a.txt", true)]);
    app.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it".into()));
    let ui = sidebar();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let arrow = hits.rect_of(&Target::Actions).expect("the caret's room");
    let icon = frame.layers()[0]
        .icons
        .iter()
        .find(|icon| icon.rect.x >= arrow.x && icon.rect.right() <= arrow.right())
        .expect("the caret");
    let left = icon.rect.x - arrow.x;
    let right = arrow.right() - icon.rect.right();
    assert!(
        (left - right).abs() <= 1.0,
        "the same room either side: {left} against {right}"
    );
}

#[test]
fn the_word_is_centred_in_its_own_part_of_the_button() {
    let mut app = listed(&[file("a.txt", true)]);
    app.workspace
        .message
        .edit(&groove_types::Edit::Insert("fix(a): it".into()));
    let ui = sidebar();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let word = hits.rect_of(&Target::Do).expect("the word");
    let arrow = hits.rect_of(&Target::Actions).expect("the caret");
    assert!(
        word.right() <= arrow.x,
        "the two parts do not overlap: {word:?} {arrow:?}"
    );
    let run = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "commit")
        .expect("the word is drawn");
    let mut fonts = Fonts::embedded();
    let style = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0))
        .small(crate::style::Role::Text);
    let wide = fonts.measure(&run.text, style.font, style.weight, style.size);
    let left = run.x - word.x;
    let right = word.right() - (run.x + wide);
    assert!(
        (left - right).abs() <= 1.0,
        "the same room either side of it: {left} against {right}"
    );
}
