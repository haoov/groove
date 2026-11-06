//! A paste read off the clipboard after the keys moved on.

use groove_controllers::{AppState, SyncSpawner};
use groove_ui::Ui;
use groove_ui::input::Input;

use super::explorer::working;

#[test]
fn a_paste_that_lands_after_the_keys_moved_is_dropped() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, _services) = working(home.path(), &spawner);
    let mut ui = Ui {
        surface: groove_ui::Surface::Session,
        focus: groove_ui::Focus::Agent,
        overlay: Some(groove_ui::Overlay::Palette(Default::default())),
        ..Ui::default()
    };
    let asked = |ui: &Ui, state: &AppState| {
        Some(crate::app::events::Unpasted {
            key: None,
            aim: groove_ui::input::aim(ui, state),
        })
    };
    let mut unpasted = asked(&ui, &state);
    ui.overlay = None;
    state.pasted = Some(Some("git@host:g/r.git".into()));
    let aim = groove_ui::input::aim(&ui, &state);
    let landed = crate::app::events::landed(&mut state, &mut unpasted, aim.clone());
    assert_eq!(landed, None, "the agent never gets the palette's text");

    let mut unpasted = asked(&ui, &state);
    state.pasted = Some(Some("one".into()));
    let landed = crate::app::events::landed(&mut state, &mut unpasted, aim);
    assert_eq!(landed, Some(Input::Paste("one".into())));
}

#[test]
fn a_paste_that_lands_after_another_session_is_selected_is_dropped() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, _services) = working(home.path(), &spawner);
    let ui = Ui {
        surface: groove_ui::Surface::Session,
        focus: groove_ui::Focus::Agent,
        ..Ui::default()
    };
    let ctrl_v = Input::Key {
        key: groove_ui::input::Key::Char('v'),
        mods: groove_ui::input::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    };
    let was = state.session.selected.clone();
    for pasted in [Some("one".to_string()), None] {
        state.session.selected = was.clone();
        let mut unpasted = Some(crate::app::events::Unpasted {
            key: Some(ctrl_v.clone()),
            aim: groove_ui::input::aim(&ui, &state),
        });
        state.session.selected = Some(groove_types::SessionId::new("another"));
        state.pasted = Some(pasted.clone());
        let aim = groove_ui::input::aim(&ui, &state);
        let landed = crate::app::events::landed(&mut state, &mut unpasted, aim);
        assert_eq!(landed, None, "{pasted:?} never reaches the other agent");
    }
}
