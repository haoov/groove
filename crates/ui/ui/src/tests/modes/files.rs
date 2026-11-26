//! The Files tab in the modes layout: the open file in the middle under its band, the worktree's
//! tree and the open files in the sidebar, the keys and the wheel on the code, and the popup.

use groove_controllers::workspace_service::from_text;
use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::{FileDiff, FileStatus, WorktreeId};

use super::moded;
use crate::hit::{Hits, Target};
use crate::input::{Input, Key, Modifiers};
use crate::layout::Layout;
use crate::tests::chat::talking;
use crate::tests::{ALT, all_texts, click, handle, press, window};
use crate::views::session::Mode;

/// The modes layout on, the worktree walked, `src/a.rs` changed, and `src/a.rs` then `b.md` open.
fn opened() -> AppState {
    let mut app = talking(Vec::new());
    on(&mut app);
    let worktree = WorktreeId::new("wt-1");
    let changed = FileDiff {
        path: "src/a.rs".into(),
        added: 2,
        deleted: 1,
        status: FileStatus::Modified,
        staged: Some(false),
    };
    app.workspace
        .loaded(worktree.clone(), vec![changed], Default::default());
    let paths = ["src/a.rs", "src/c.rs", "b.md"].map(|path| FileDiff {
        path: path.into(),
        added: 0,
        deleted: 0,
        status: FileStatus::Unchanged,
        staged: None,
    });
    app.workspace.set_paths(paths.to_vec());
    for (path, text) in [("src/a.rs", "alphaline\n"), ("b.md", "betaline\n")] {
        let file = from_text(path, text, text);
        app.workspace.arrived(&worktree, file, None, true);
        app.workspace.keep(path);
    }
    app
}

fn on(app: &mut AppState) {
    let config = app.config.config.as_mut().expect("a config");
    config.ui.modes = true;
}

/// The ui on `FILES`, settled.
fn filing(app: &AppState) -> crate::Ui {
    let mut ui = moded();
    ui.session.mode = Mode::Files;
    ui.settle(app);
    ui
}

fn texts(app: &AppState, ui: &crate::Ui) -> (Vec<String>, Hits) {
    let (frame, hits) = crate::view(app, ui, window(), &mut Fonts::embedded());
    (all_texts(&frame), hits)
}

fn open(path: &str) -> Command {
    let path = path.into();
    Command::Workspace(workspace::Command::OpenFile { path, at: None })
}

#[test]
fn the_middle_shows_the_open_file_under_its_band_and_no_strip_of_tabs() {
    let app = opened();
    let ui = filing(&app);
    let (said, hits) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "b.md"), "the band: {said:?}");
    assert!(said.iter().any(|one| one.contains("betaline")), "{said:?}");
    assert!(!said.iter().any(|one| one.contains("comes to this layout")));
    let sidebar = Layout::of(window(), &ui).sidebar;
    for path in ["src/a.rs", "b.md"] {
        let row = hits
            .rect_of(&Target::OpenTab(path.into()))
            .expect("an open row");
        assert!(sidebar.contains(row.x + 1.0, row.y + 1.0), "in the sidebar");
    }
}

#[test]
fn a_click_on_a_file_of_the_tree_opens_it_in_the_middle() {
    let app = opened();
    let mut ui = filing(&app);
    let (said, hits) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "FILES · kiwi"), "{said:?}");
    let row = hits
        .rect_of(&Target::File("b.md".into()))
        .expect("the tree's row");
    let asked = click(row, &mut ui, &app, &hits);
    assert!(asked.contains(&open("b.md")), "{asked:?}");
    ui.settle(&app);
    assert_eq!(ui.session.mode, Mode::Files);
}

#[test]
fn the_open_rows_list_the_open_files_and_a_click_shows_one() {
    let app = opened();
    let mut ui = filing(&app);
    let (said, hits) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "OPEN · 2"), "{said:?}");
    let row = hits
        .rect_of(&Target::OpenTab("src/a.rs".into()))
        .expect("its open row");
    let asked = click(row, &mut ui, &app, &hits);
    assert!(asked.contains(&open("src/a.rs")), "{asked:?}");
}

#[test]
fn the_middle_button_or_the_menu_closes_an_open_row() {
    let app = opened();
    let mut ui = filing(&app);
    let (_, hits) = texts(&app, &ui);
    let row = hits
        .rect_of(&Target::OpenTab("b.md".into()))
        .expect("its open row");
    let (x, y) = (row.x + row.w / 3.0, row.y + row.h / 2.0);
    let asked = handle(Input::Middle { x, y }, &mut ui, &app, &hits, window());
    let close = workspace::Command::CloseFile {
        path: "b.md".into(),
    };
    assert_eq!(asked, [Command::Workspace(close.clone())]);
    handle(Input::Menu { x, y }, &mut ui, &app, &hits, window());
    let (said, _) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "close"), "{said:?}");
    let asked = crate::tests::hit(&Target::MenuRow(0), &mut ui, &app);
    assert_eq!(asked, [Command::Workspace(close)]);
}

#[test]
fn a_directory_of_the_tree_opens_and_folds() {
    let app = opened();
    let mut ui = filing(&app);
    let (_, hits) = texts(&app, &ui);
    assert!(hits.rect_of(&Target::File("src/c.rs".into())).is_none());
    let dir = hits.rect_of(&Target::Dir("src".into())).expect("its row");
    click(dir, &mut ui, &app, &hits);
    let (said, hits) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "c.rs"), "{said:?}");
    let dir = hits.rect_of(&Target::Dir("src".into())).expect("its row");
    click(dir, &mut ui, &app, &hits);
    let (_, hits) = texts(&app, &ui);
    assert!(hits.rect_of(&Target::File("src/c.rs".into())).is_none());
}

#[test]
fn alt_o_on_files_gives_the_keys_to_the_code() {
    let app = opened();
    let mut ui = filing(&app);
    ui.focus = crate::Focus::Rail;
    press(Key::Char('o'), ALT, &mut ui, &app);
    assert_eq!(ui.focus, crate::Focus::Workspace);
}

#[test]
fn the_arrows_walk_the_open_rows_then_the_tree_as_drawn() {
    let app = opened();
    let mut ui = filing(&app);
    ui.focus = crate::Focus::Sidebar;
    let (_, hits) = texts(&app, &ui);
    let mut walked = Vec::new();
    for _ in 0..8 {
        let input = Input::Key {
            key: Key::Up,
            mods: Modifiers::default(),
        };
        handle(input, &mut ui, &app, &hits, window());
        let at = ui.hover.clone().expect("a row is stood on");
        assert!(hits.rect_of(&at).is_some(), "a drawn row: {at:?}");
        if walked.last() != Some(&at) {
            walked.push(at);
        }
    }
    assert_eq!(
        walked,
        [
            Target::File("b.md".into()),
            Target::Dir("src".into()),
            Target::OpenTab("b.md".into()),
            Target::OpenTab("src/a.rs".into()),
        ],
        "up from the open file in the tree, past the head, to the open rows over it"
    );
}

#[test]
fn the_wheel_over_the_open_file_scrolls_it() {
    let mut app = opened();
    let long: String = (0..300).map(|at| format!("line {at}\n")).collect();
    let worktree = WorktreeId::new("wt-1");
    let file = from_text("long.rs", &long, &long);
    app.workspace.arrived(&worktree, file, None, true);
    let mut ui = filing(&app);
    let (_, hits) = texts(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let wheel = Input::Scroll {
        x: code.x + code.w / 2.0,
        y: code.y + code.h / 2.0,
        delta: crate::input::Delta::Pixels {
            across: 0.0,
            down: -code.h,
        },
    };
    handle(wheel, &mut ui, &app, &hits, window());
    assert!(ui.session.file > 0.0, "the rows scrolled");
}

#[test]
fn a_fault_s_popup_heads_on_its_kind_and_line() {
    use crate::tests::diagnostics::{PATH, diagnosed};
    let mut app = diagnosed(groove_types::Severity::Warning);
    crate::tests::chat::configured(&mut app);
    on(&mut app);
    let at = groove_types::Caret::new(0, 5);
    app.workspace.servers.asked(PATH, at, false);
    let answer = Some("The absence of a value.".into());
    app.workspace.servers.answered(PATH, at, answer);
    let mut ui = filing(&app);
    crate::tests::hover::moved_to(&mut ui, &app, at, window());
    crate::input::hovering(&mut ui, &app, 0);
    let mut drawn = window();
    drawn.tick = 300;
    let (frame, _) = crate::view(&app, &ui, drawn, &mut Fonts::embedded());
    let said = all_texts(&frame);
    let place = |text: &str| said.iter().position(|one| one == text);
    let (warning, hover) = (place("WARNING · LINE 1"), place("HOVER · LINE 1"));
    assert!(warning.is_some() && hover.is_some(), "{said:?}");
    assert!(warning < hover, "the fault first: {said:?}");
}

