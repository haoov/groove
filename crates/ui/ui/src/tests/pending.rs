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
