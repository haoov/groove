//! The Diff tab in the modes layout: one file at a time in the middle, the change's files in
//! the sidebar as a tree or a list, the wheel on the code, and a cluster write answered first.

use groove_controllers::workspace_service::{Changes, aligned, prepare};
use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::{ApprovalId, FileDiff, FileStatus, WorktreeId};

use super::moded;
use crate::hit::{Hits, Target};
use crate::input::{Delta, Input};
use crate::tests::agent::{ask, asking};
use crate::tests::chat::talking;
use crate::tests::{all_texts, click, handle, window};
use crate::views::session::Mode;

/// The modes layout switched on in the config, the change of `files` read for the session's worktree.
fn changed(files: &[(&str, &str, &str)]) -> AppState {
    let mut app = talking(Vec::new());
    on(&mut app);
    let read = files
        .iter()
        .map(|(path, a, b)| aligned(path, a, b))
        .collect();
    let listed = files.iter().map(|(path, ..)| FileDiff {
        path: (*path).to_string(),
        added: 2,
        deleted: 1,
        status: FileStatus::Modified,
        staged: Some(false),
    });
    let worktree = WorktreeId::new("wt-1");
    app.workspace
        .loaded(worktree, listed.collect(), Changes::new(read));
    app
}

fn on(app: &mut AppState) {
    let config = app.config.config.as_mut().expect("a config");
    config.ui.modes = true;
}

/// The ui on `DIFF`, settled, and the focus the frame asks for applied.
fn diffing(app: &mut AppState) -> crate::Ui {
    let mut ui = moded();
    ui.session.mode = Mode::Diff;
    ui.settle(app);
    settled(app, &ui);
    ui
}

fn settled(app: &mut AppState, ui: &crate::Ui) {
    let asked = crate::views::session::files::moded::focused(app, ui);
    if let Some(Command::Workspace(workspace::Command::Focus { path })) = asked {
        app.workspace.changes.focus(path);
    }
}

fn texts(app: &AppState, ui: &crate::Ui) -> (Vec<String>, Hits) {
    let (frame, hits) = crate::view(app, ui, window(), &mut Fonts::embedded());
    (all_texts(&frame), hits)
}

fn two() -> AppState {
    changed(&[
        ("src/b/beta.rs", "betaold\n", "betanew\n"),
        ("src/a/alpha.rs", "alphaold\n", "alphanew\n"),
    ])
}

#[test]
fn the_middle_shows_the_first_file_alone_when_none_is_chosen() {
    let mut app = two();
    let ui = diffing(&mut app);
    assert_eq!(app.workspace.changes.focused(), Some("src/a/alpha.rs"));
    let (said, _) = texts(&app, &ui);
    assert!(said.iter().any(|one| one.contains("alphanew")), "{said:?}");
    assert!(!said.iter().any(|one| one.contains("betanew")), "{said:?}");
    assert!(!said.iter().any(|one| one.contains("comes to this layout")));
}

#[test]
fn a_click_on_a_file_in_the_sidebar_shows_it_in_the_middle() {
    let mut app = two();
    let mut ui = diffing(&mut app);
    let (_, hits) = texts(&app, &ui);
    let row = hits
        .rect_of(&Target::File("src/b/beta.rs".into()))
        .expect("its row");
    let asked = click(row, &mut ui, &app, &hits);
    let path = Some("src/b/beta.rs".to_string());
    assert!(
        asked.contains(&Command::Workspace(workspace::Command::Focus {
            path: path.clone()
        }))
    );
    app.workspace.changes.focus(path);
    settled(&mut app, &ui);
    let (said, _) = texts(&app, &ui);
    assert!(said.iter().any(|one| one.contains("betanew")), "{said:?}");
    assert!(!said.iter().any(|one| one.contains("alphanew")), "{said:?}");
}

#[test]
fn the_sidebar_s_head_holds_the_whole_change_s_totals() {
    let mut app = two();
    let mut ui = diffing(&mut app);
    ui.split.sidebar = crate::tests::WIDE_SIDEBAR;
    let (said, _) = texts(&app, &ui);
    for one in ["DIFF · 2 files", "+4", "-2"] {
        assert!(said.iter().any(|text| text == one), "{one}: {said:?}");
    }
    assert!(said.iter().any(|text| text == "FILES · 2"), "{said:?}");
}

#[test]
fn the_list_switches_between_tree_and_list_by_its_right_click() {
    let mut app = two();
    let mut ui = diffing(&mut app);
    let (said, hits) = texts(&app, &ui);
    assert!(
        said.iter().any(|one| one == "src"),
        "a directory row: {said:?}"
    );
    let row = hits
        .rect_of(&Target::File("src/a/alpha.rs".into()))
        .expect("its row");
    let menu = Input::Menu {
        x: row.x + row.w / 2.0,
        y: row.y + row.h / 2.0,
    };
    handle(menu, &mut ui, &app, &hits, window());
    let (said, _) = texts(&app, &ui);
    assert!(said.iter().any(|one| one == "Show as a list"), "{said:?}");
    crate::tests::hit(&Target::MenuRow(0), &mut ui, &app);
    assert!(ui.session.flat);
    let (said, _) = texts(&app, &ui);
    assert!(
        !said.iter().any(|one| one == "src"),
        "no directory row: {said:?}"
    );
    assert!(
        said.iter().any(|one| one == "src/a"),
        "its folder: {said:?}"
    );
}

#[test]
fn a_directory_of_the_tree_folds_its_files() {
    let mut app = two();
    let mut ui = diffing(&mut app);
    let (_, hits) = texts(&app, &ui);
    let dir = hits
        .rect_of(&Target::Group("src/a".into()))
        .expect("its row");
    click(dir, &mut ui, &app, &hits);
    let (said, hits) = texts(&app, &ui);
    assert!(
        hits.rect_of(&Target::File("src/a/alpha.rs".into()))
            .is_none()
    );
    assert!(
        hits.rect_of(&Target::File("src/b/beta.rs".into()))
            .is_some()
    );
    assert!(said.iter().any(|one| one == "1 file"), "{said:?}");
}

#[test]
fn the_wheel_over_the_middle_scrolls_the_code() {
    let before: String = (0..300).map(|at| format!("line {at}\n")).collect();
    let after = before.to_uppercase();
    let mut app = changed(&[("src/long.rs", &before, &after)]);
    let mut ui = diffing(&mut app);
    let (_, hits) = texts(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let wheel = Input::Scroll {
        x: code.x + code.w / 2.0,
        y: code.y + code.h / 2.0,
        delta: Delta::Pixels {
            across: 0.0,
            down: -code.h,
        },
    };
    handle(wheel, &mut ui, &app, &hits, window());
    assert!(ui.session.diff > 0.0, "the rows scrolled");
}

#[test]
fn a_cluster_write_takes_the_session_to_diff_and_back_once_answered() {
    let mut one = ask("ap-1", "kube_patch", "kind · paxone/deployments/api");
    one.reviewed = true;
    let mut app = asking(vec![one]);
    crate::tests::chat::configured(&mut app);
    on(&mut app);
    let before = "spec:\n  replicas: 1\n".to_string();
    let after = "spec:\n  replicas: 3\n".to_string();
    let pairs = vec![("kind/paxone/deployments/api.yaml".into(), before, after)];
    app.workspace
        .review(&ApprovalId::new("ap-1"), prepare(pairs));
    let mut ui = moded();
    ui.settle(&app);
    assert_eq!(ui.session.mode, Mode::Diff);
    settled(&mut app, &ui);
    let (said, hits) = texts(&app, &ui);
    assert!(
        said.iter().filter(|one| *one == "replicas").count() >= 2,
        "{said:?}"
    );
    assert!(
        hits.rect_of(&Target::Approve(ApprovalId::new("ap-1")))
            .is_some()
    );
    app.agent = asking(Vec::new()).agent;
    ui.settle(&app);
    assert_eq!(ui.session.mode, Mode::Agent, "back to the mode it stood on");
}
