//! The sidebar's search bar: where it opens, and what a path or a text narrows.

use super::*;

#[test]
fn a_chord_opens_the_search_bar_and_what_is_typed_narrows_the_list() {
    let app = with_files(&["src/one/alpha.rs", "src/two/beta.rs", "README.md"]);
    let mut ui = on_diff();
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    assert!(
        ui.session.bar.typing.is_some(),
        "the keyboard is in the bar"
    );

    for c in "two/be".chars() {
        press(
            Key::Char(c),
            crate::input::Modifiers::default(),
            &mut ui,
            &app,
        );
    }
    let left = crate::views::session::files::narrowed(&app, &ui);
    let paths: Vec<&str> = left.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["src/two/beta.rs"], "the one path that matches");

    let commands = press(
        Key::Enter,
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert_eq!(commands.len(), 1, "it opens what is left");
    assert_eq!(commands[0].id(), "workspace.open_file");
    assert!(ui.session.bar.typing.is_none(), "and the bar is spent");
    assert_eq!(ui.session.bar.path.text(), "two/be", "what was typed stays");
}

#[test]
fn escape_leaves_the_list_as_it_was() {
    let app = with_files(&["src/one/alpha.rs", "src/two/beta.rs"]);
    let mut ui = on_diff();
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    press(
        Key::Char('z'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert_eq!(
        crate::views::session::files::narrowed(&app, &ui).len(),
        0,
        "nothing matches z"
    );
    press(
        Key::Escape,
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert!(ui.session.bar.typing.is_none());
    assert_eq!(
        crate::views::session::files::narrowed(&app, &ui).len(),
        2,
        "both files are back"
    );
}

#[test]
fn the_chord_shows_the_bar_it_opens_from_a_tab_that_has_no_sidebar() {
    let app = with_files(&["src/one/alpha.rs", "src/two/beta.rs"]);
    let mut ui = Ui::default();
    ui.session.tab = Tab::Overview;
    ui.focus = crate::Focus::Workspace;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    assert!(
        ui.session.sidebar(),
        "the bar the chord opened is on screen"
    );
    assert!(ui.session.bar.typing.is_some(), "and it has the keyboard");

    for c in "two".chars() {
        press(
            Key::Char(c),
            crate::input::Modifiers::default(),
            &mut ui,
            &app,
        );
    }
    let left = crate::views::session::files::narrowed(&app, &ui);
    let paths: Vec<&str> = left.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["src/two/beta.rs"]);
}

#[test]
fn the_chord_unfolds_a_sidebar_that_was_folded_away() {
    let app = with_files(&["src/one/alpha.rs"]);
    let mut ui = on_diff();
    ui.session.folded = true;
    ui.focus = crate::Focus::Workspace;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    assert!(!ui.session.folded, "the sidebar is back");
    assert!(ui.session.bar.typing.is_some());
}

#[test]
fn the_board_holds_no_file_list_so_the_chord_takes_nothing() {
    let app = with_files(&["src/one/alpha.rs"]);
    let mut ui = Ui {
        surface: crate::Surface::Board,
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    assert!(
        ui.session.bar.typing.is_none(),
        "nothing invisible took the keyboard"
    );
}

/// The worktree's own files, as the walk leaves them in the slice.
fn with_paths(app: &mut AppState, paths: &[&str]) {
    app.workspace.paths = paths
        .iter()
        .map(|path| FileDiff {
            path: (*path).into(),
            added: 0,
            deleted: 0,
            status: FileStatus::Unchanged,
            staged: None,
        })
        .collect();
}

#[test]
fn a_path_finds_a_file_that_never_changed() {
    let mut app = with_files(&["src/one/alpha.rs"]);
    with_paths(
        &mut app,
        &["src/one/alpha.rs", "src/two/beta.rs", "README.md"],
    );
    let mut ui = on_diff();
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    for c in "beta".chars() {
        press(
            Key::Char(c),
            crate::input::Modifiers::default(),
            &mut ui,
            &app,
        );
    }
    let left = crate::views::session::files::narrowed(&app, &ui);
    let paths: Vec<&str> = left.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["src/two/beta.rs"], "a file outside the diff");
}

#[test]
fn a_changed_file_stands_before_an_unchanged_one_and_only_once() {
    let mut app = with_files(&["src/one/alpha.rs"]);
    with_paths(&mut app, &["src/one/alpha.rs", "src/one/alpha_test.rs"]);
    let mut ui = on_diff();
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    for c in "alpha".chars() {
        press(
            Key::Char(c),
            crate::input::Modifiers::default(),
            &mut ui,
            &app,
        );
    }
    let left = crate::views::session::files::narrowed(&app, &ui);
    let paths: Vec<&str> = left.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["src/one/alpha.rs", "src/one/alpha_test.rs"]);
    assert_eq!(
        left[0].status,
        FileStatus::Modified,
        "the changed one first"
    );
    assert_eq!(left[1].status, FileStatus::Unchanged);
}

#[test]
fn the_first_character_of_a_path_asks_for_the_worktrees_files() {
    let app = with_files(&["src/one/alpha.rs"]);
    let mut ui = on_diff();
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    let asked = press(
        Key::Char('a'),
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    let ids: Vec<&str> = asked.iter().map(|one| one.id()).collect();
    assert_eq!(ids, ["workspace.list_paths"]);
}

#[test]
fn a_file_with_no_change_offers_nothing_to_stage() {
    let mut app = with_files(&[]);
    with_paths(&mut app, &["src/two/beta.rs"]);
    let mut ui = on_diff();
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    for c in "beta".chars() {
        press(
            Key::Char(c),
            crate::input::Modifiers::default(),
            &mut ui,
            &app,
        );
    }
    ui.hover = Some(Target::File("src/two/beta.rs".into()));
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Stage("src/two/beta.rs".into()))
            .is_none(),
        "there is nothing of it to stage"
    );
}

#[test]
fn a_file_with_no_change_shows_its_own_lines() {
    let mut app = with_files(&[]);
    with_paths(&mut app, &["src/two/beta.rs"]);
    app.workspace.opened = Some(groove_controllers::workspace_service::from_text(
        "src/two/beta.rs",
        "one\ntwo\n",
        "one\ntwo\n",
    ));
    let mut ui = on_diff();
    ui.session.view = groove_types::DiffView::File;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.iter())
        .map(|t| t.text.clone())
        .collect();
    assert!(
        !texts.iter().any(|t| t.contains("No change in this")),
        "the tab does not claim there is nothing: {texts:?}"
    );
    let grids: usize = frame.layers().iter().map(|layer| layer.grids.len()).sum();
    assert!(
        texts.iter().any(|t| t == "src/two/beta.rs") || grids > 0,
        "the file is named and drawn: {texts:?}"
    );
}

#[test]
fn opening_a_file_the_change_does_not_hold_shows_it_as_a_file() {
    let mut app = with_files(&["src/one/alpha.rs"]);
    with_paths(&mut app, &["src/two/beta.rs"]);
    let mut ui = on_diff();
    ui.session.view = groove_types::DiffView::Inline;
    ui.focus = crate::Focus::Sidebar;
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    for c in "beta".chars() {
        press(
            Key::Char(c),
            crate::input::Modifiers::default(),
            &mut ui,
            &app,
        );
    }
    let asked = press(
        Key::Enter,
        crate::input::Modifiers::default(),
        &mut ui,
        &app,
    );
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].id(), "workspace.open_file");
    assert_eq!(
        ui.session.view,
        groove_types::DiffView::File,
        "an unchanged file has only one view that can show it"
    );
}
