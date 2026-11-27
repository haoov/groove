//! The Kubernetes tab from the keyboard: the kinds, the list's rows, the open objects.

use super::*;
use crate::Focus;
use crate::input::{Key, Modifiers};
use crate::views::session::resources::View;

fn key(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) {
    crate::tests::press(key, mods, ui, app);
}

#[test]
fn the_list_s_rows_walk_and_open_on_describe_yaml_or_logs() {
    let (app, mut ui) = listing();
    ui.focus = Focus::Workspace;
    let rows = crate::views::session::resources::row_uids(&app, &ui);
    key(Key::Down, Modifiers::default(), &mut ui, &app);
    key(Key::Down, Modifiers::default(), &mut ui, &app);
    assert_eq!(ui.hover, Some(Target::ResourceRow(rows[1].clone())));
    for (pressed, view) in [
        (Key::Enter, View::Describe),
        (Key::Char('y'), View::Yaml),
        (Key::Char('l'), View::Logs),
    ] {
        ui.session.resources.showing = None;
        key(pressed, Modifiers::default(), &mut ui, &app);
        let tab = ui.session.resources.tab().expect("an object opened");
        assert_eq!(tab.view, view, "{pressed:?}");
    }
}

#[test]
fn enter_in_the_list_s_search_stands_on_its_first_row() {
    let (app, mut ui) = listing();
    ui.focus = Focus::Workspace;
    (ui.session.resources.finding, ui.session.resources.typing) = (true, true);
    crate::tests::typed("worker", &mut ui, &app);
    key(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(!ui.session.resources.typing);
    assert_eq!(ui.hover, Some(Target::ResourceRow("shop/worker-0".into())));
}

#[test]
fn ctrl_tab_walks_the_open_objects_and_ctrl_w_closes_one() {
    let (app, mut ui) = listing();
    ui.focus = Focus::Workspace;
    for _ in 0..2 {
        ui.session.resources.showing = None;
        key(Key::Down, Modifiers::default(), &mut ui, &app);
        key(Key::Enter, Modifiers::default(), &mut ui, &app);
    }
    assert_eq!(ui.session.resources.opened.len(), 2);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    key(Key::Tab, ctrl, &mut ui, &app);
    assert_eq!(
        ui.session.resources.showing, None,
        "the list after the last"
    );
    key(Key::Tab, ctrl, &mut ui, &app);
    assert_eq!(ui.session.resources.showing, Some(0));
    key(Key::Char('w'), ctrl, &mut ui, &app);
    assert_eq!(ui.session.resources.opened.len(), 1);
}
