//! The Files tab's strip: the preview in italic, a tab's menu, the middle button, a double click.

use groove_controllers::workspace_service::from_text;
use groove_controllers::{Command, workspace};
use groove_gfx::Font;
use groove_types::Edit;

use super::*;
use crate::Losing;
use crate::input::{Input, handle};

/// Three tabs: `a.rs` kept, `b.rs` with edits, `c.rs` the preview and the active one.
fn tabbed() -> (AppState, Ui) {
    let mut app = with_files(&["a.rs", "b.rs", "c.rs"]);
    let worktree = app.workspace.worktree.clone().expect("a worktree");
    for path in ["a.rs", "b.rs", "c.rs"] {
        let file = from_text(path, "one\n", "one\n");
        app.workspace.arrived(&worktree, file, None, true);
        match path {
            "a.rs" => app.workspace.keep(path),
            "b.rs" => drop(app.workspace.edit(&Edit::Insert("x".into()))),
            _ => {}
        }
    }
    let mut ui = Ui::default();
    ui.session.tab = Tab::Files;
    (app, ui)
}

fn close(path: &str) -> Command {
    Command::Workspace(workspace::Command::CloseFile { path: path.into() })
}

/// The menu of `path`'s tab, row `at` picked.
fn picked(app: &AppState, ui: &mut Ui, path: &str, at: usize) -> Vec<Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let tab = hits
        .rect_of(&Target::OpenTab(path.into()))
        .expect("the tab");
    let (x, y) = (tab.x + tab.w / 3.0, tab.y + tab.h / 2.0);
    handle(Input::Menu { x, y }, ui, app, &hits, window());
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let row = hits.rect_of(&Target::MenuRow(at)).expect("the row");
    click(row, ui, app, &hits)
}

#[test]
fn the_preview_s_name_is_in_italic_and_the_kept_ones_are_not() {
    let (app, ui) = tabbed();
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let font = |name: &str| {
        let runs = frame.layers().iter().flat_map(|layer| layer.texts.iter());
        runs.filter(|run| run.text == name)
            .map(|run| run.style.font)
            .next()
    };
    assert_eq!(font("c.rs"), Some(Font::Italic));
    assert_ne!(font("a.rs"), Some(Font::Italic));
}

#[test]
fn closing_the_others_leaves_the_tabs_that_owe_the_disk() {
    let (app, mut ui) = tabbed();
    let rows = picked(&app, &mut ui, "a.rs", 1);
    assert_eq!(rows, [close("c.rs")], "b.rs keeps its edits");
}

#[test]
fn closing_all_takes_the_clicked_tab_too() {
    let (app, mut ui) = tabbed();
    assert_eq!(
        picked(&app, &mut ui, "a.rs", 2),
        [close("c.rs"), close("a.rs")]
    );
}

#[test]
fn closing_a_tab_with_edits_asks_first() {
    let (app, mut ui) = tabbed();
    assert!(picked(&app, &mut ui, "b.rs", 0).is_empty());
    assert_eq!(ui.losing(), Some(&Losing::Tab("b.rs".into())));
}

#[test]
fn the_middle_button_closes_a_tab() {
    let (app, mut ui) = tabbed();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let tab = hits
        .rect_of(&Target::OpenTab("a.rs".into()))
        .expect("the tab");
    let (x, y) = (tab.x + tab.w / 3.0, tab.y + tab.h / 2.0);
    let asked = handle(Input::Middle { x, y }, &mut ui, &app, &hits, window());
    assert_eq!(asked, [close("a.rs")]);
}

#[test]
fn a_double_click_on_a_tab_keeps_it() {
    let (app, mut ui) = tabbed();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let tab = hits
        .rect_of(&Target::OpenTab("c.rs".into()))
        .expect("the tab");
    let open = Command::Workspace(workspace::Command::OpenFile {
        path: "c.rs".into(),
        at: None,
    });
    assert_eq!(
        click(tab, &mut ui, &app, &hits),
        std::slice::from_ref(&open),
        "once"
    );
    let keep = Command::Workspace(workspace::Command::KeepFile {
        path: "c.rs".into(),
    });
    assert_eq!(click(tab, &mut ui, &app, &hits), [open, keep], "twice");
}
