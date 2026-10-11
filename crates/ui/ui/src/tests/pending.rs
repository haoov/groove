//! What waits on the user ends with what it was about: a session left, a surface hidden, Settings shut.

use groove_types::SessionId;

use crate::keymap::Action;
use crate::tests::full_app;
use crate::views::session::Tab;
use crate::{Losing, Overlay, Ui};

#[test]
fn a_question_about_a_session_left_behind_goes_with_it() {
    let mut app = full_app();
    let mut ui = Ui::default();
    ui.settle(&app);
    ui.overlay = Some(Overlay::Losing(Losing::Everything));
    ui.session.going = Some((Tab::Diff, None));
    ui.settle(&app);
    assert!(ui.overlay.is_some(), "it stands while nothing moves");

    app.session.selected = Some(SessionId::new("another"));
    ui.settle(&app);
    assert_eq!(
        ui.overlay, None,
        "discard all never reaches the other session"
    );
    assert_eq!(ui.session.going, None);
}

#[test]
fn settings_shut_while_binding_take_no_chord_after() {
    let mut ui = Ui::default();
    ui.settings.open = true;
    ui.settings.binding = Some(Action::Palette);
    ui.close_settings();
    assert_eq!(ui.settings.binding, None);
}

#[test]
fn a_folded_sidebar_lets_go_of_the_keys() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.settle(&app);
    (ui.focus, ui.session.composing) = (crate::Focus::Sidebar, true);
    ui.session.folded = true;
    ui.settle(&app);
    assert_eq!(ui.focus, crate::Focus::Workspace);
    assert!(!ui.session.composing, "the hidden commit box takes no keys");
}

#[test]
fn find_matches_follow_an_edit_of_the_file() {
    use crate::input::{Key, Modifiers};
    let mut app = full_app();
    crate::tests::shows(&mut app, "src/a.rs", "let one = 1;\n", "let one = 1;\n");
    let mut ui = Ui::default();
    (ui.session.tab, ui.focus) = (Tab::Files, crate::Focus::Workspace);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    crate::tests::press(Key::Char('f'), ctrl, &mut ui, &app);
    for c in "one".chars() {
        crate::tests::press(Key::Char(c), Modifiers::default(), &mut ui, &app);
    }
    let rows = |ui: &Ui| {
        ui.session
            .find
            .as_ref()
            .map(|one| one.hits.iter().map(|hit| hit.row).collect::<Vec<_>>())
    };
    assert_eq!(rows(&ui), Some(vec![0]));
    app.workspace.edit(&groove_types::Edit::Newline);
    ui.settle(&app);
    assert_eq!(
        rows(&ui),
        Some(vec![1]),
        "the match moved down with its line"
    );
}

#[test]
fn a_tab_waiting_on_a_commit_that_failed_to_read_stops_waiting_and_stays() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.settle(&app);
    ui.session.going = Some((Tab::Diff, Some("0123456".into())));
    ui.settle(&app);
    assert_eq!(ui.session.going, None);
    assert_ne!(ui.session.tab, Tab::Diff, "nothing to show there");
}
