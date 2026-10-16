//! The sidebar from the keyboard: the arrows, Enter, Space, the folds, the lists, the box.

use groove_controllers::{Command, workspace};
use groove_types::{CommitEntry, Timestamp};

use super::*;
use crate::Focus;
use crate::input::Modifiers;
use crate::tests::{ALT, sidebar_ui};
use crate::views::session::Pane;

fn key(key: Key, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    press(key, Modifiers::default(), ui, app)
}

#[test]
fn the_arrows_walk_the_changed_files_and_space_stages_one() {
    let app = with_files(&["a.rs", "b.rs"]);
    let mut ui = sidebar_ui();
    let moved = key(Key::Down, &mut ui, &app);
    assert_eq!(ui.hover, Some(Target::File("a.rs".into())));
    assert!(moved.is_empty(), "the arrows only stand on a file");
    key(Key::Down, &mut ui, &app);
    assert_eq!(ui.hover, Some(Target::File("b.rs".into())));
    let staged = key(Key::Char(' '), &mut ui, &app);
    assert!(matches!(
        &staged[..],
        [Command::Workspace(workspace::Command::Stage { path })] if path == "b.rs"
    ));
    key(Key::Enter, &mut ui, &app);
    assert_eq!(
        ui.focus,
        Focus::Workspace,
        "Enter hands the keys to the code"
    );
}

#[test]
fn in_the_tree_left_and_right_fold_a_folder_and_enter_opens_a_file() {
    let (app, mut ui) = super::explorer::browsing(&["src/one.rs", "top.rs"]);
    ui.focus = Focus::Sidebar;
    key(Key::Down, &mut ui, &app);
    assert_eq!(ui.hover, Some(Target::Dir("src".into())));
    key(Key::Right, &mut ui, &app);
    assert!(ui.session.opened.contains("src"), "opened");
    key(Key::Down, &mut ui, &app);
    assert_eq!(
        ui.hover,
        Some(Target::File("src/one.rs".into())),
        "into the folder"
    );
    key(Key::Up, &mut ui, &app);
    key(Key::Left, &mut ui, &app);
    assert!(!ui.session.opened.contains("src"), "shut");
    key(Key::Down, &mut ui, &app);
    assert_eq!(ui.hover, Some(Target::File("top.rs".into())));
    let opened = key(Key::Enter, &mut ui, &app);
    assert!(
        opened.iter().any(|one| one.id() == "workspace.open_file"),
        "{opened:?}"
    );
}

#[test]
fn the_commits_and_the_lists_are_walked_too() {
    let mut app = with_files(&["a.rs"]);
    app.workspace.log = vec![CommitEntry {
        sha: "abc".into(),
        short_sha: "abc".into(),
        message: "feat: one".into(),
        author: "T".into(),
        at: Timestamp::new(0),
        is_base: false,
    }];
    let mut ui = sidebar_ui();
    key(Key::Char(']'), &mut ui, &app);
    assert_eq!(ui.session.pane, Pane::Commits);
    key(Key::Down, &mut ui, &app);
    assert_eq!(ui.hover, Some(Target::Commit("abc".into())));
    let shown = key(Key::Enter, &mut ui, &app);
    assert_eq!(shown[0].id(), "workspace.open_commit");
    key(Key::Char('['), &mut ui, &app);
    assert_eq!(ui.session.pane, Pane::Files);
}

#[test]
fn alt_4_leaves_the_commit_box_and_alt_c_comes_back_to_it() {
    let app = with_files(&["a.rs"]);
    let mut ui = sidebar_ui();
    ui.session.composing = true;
    press(Key::Char('4'), ALT, &mut ui, &app);
    assert!(!ui.session.composing && ui.focus == Focus::Sidebar && !ui.session.folded);
    key(Key::Down, &mut ui, &app);
    assert_eq!(
        ui.hover,
        Some(Target::File("a.rs".into())),
        "the arrows walk the list"
    );
    press(Key::Char('c'), ALT, &mut ui, &app);
    assert!(ui.session.composing && ui.focus == Focus::Sidebar);
}

#[test]
fn a_click_on_a_row_of_the_list_takes_the_keys_out_of_the_commit_box() {
    let app = with_files(&["a.rs"]);
    let mut ui = sidebar_ui();
    ui.session.composing = true;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits.rect_of(&Target::File("a.rs".into())).expect("the row");
    click(row, &mut ui, &app, &hits);
    assert!(!ui.session.composing, "the box lets go");
    assert_eq!(ui.focus, Focus::Sidebar);
}

#[test]
fn while_the_path_is_typed_the_arrows_walk_its_matches() {
    let app = with_files(&["src/alpha.rs", "src/beta.rs"]);
    let mut ui = sidebar_ui();
    ui.session.bar.open(crate::views::session::Term::Path);
    crate::tests::typed("rs", &mut ui, &app);
    key(Key::Down, &mut ui, &app);
    key(Key::Down, &mut ui, &app);
    assert_eq!(
        ui.hover,
        Some(Target::File("src/beta.rs".into())),
        "the second match"
    );
    assert!(ui.session.bar.typing.is_some(), "the bar keeps the keys");
    let opened = key(Key::Enter, &mut ui, &app);
    assert!(
        opened.iter().any(|one| one.id() == "workspace.open_file"),
        "{opened:?}"
    );
    assert!(ui.session.bar.typing.is_none(), "Enter leaves the bar");
}

#[test]
fn ctrl_tab_walks_the_open_files_and_ctrl_w_closes_the_active_one() {
    let mut app = crate::tests::app();
    crate::tests::shows(&mut app, "a.rs", "a\n", "a\nb\n");
    crate::tests::shows(&mut app, "b.rs", "c\n", "c\nd\n");
    let mut ui = Ui::default();
    (ui.session.tab, ui.focus) = (Tab::Files, Focus::Workspace);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    let next = press(Key::Tab, ctrl, &mut ui, &app);
    assert!(
        next.iter().any(|one| one.id() == "workspace.open_file"),
        "{next:?}"
    );
    let closed = press(Key::Char('w'), ctrl, &mut ui, &app);
    assert!(
        closed.iter().any(|one| one.id() == "workspace.close_file"),
        "{closed:?}"
    );
}
