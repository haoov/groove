//! The keymap: its rings, the keys it leaves to a terminal, and a chord rebound from Settings.

use groove_controllers::{Command, agent, config};
use groove_gfx::Fonts;
use groove_types::Chord;

use super::*;
use crate::hit::Target;
use crate::keymap::{Action, Keymap};
use crate::view;
use crate::views::session::Tab;

fn chord(text: &str) -> Chord {
    Chord::parse(text).expect(text)
}

fn bare_config() -> groove_types::Config {
    let file = serde_json::json!({ "git": { "worktree_root": "~" } });
    serde_json::from_value(file).expect("a config")
}

#[test]
fn an_alt_key_groove_does_not_claim_reaches_the_agent() {
    let app = full_app();
    let mut ui = Ui::default();
    let sent = press(Key::Char('b'), ALT, &mut ui, &app);
    let Some(Command::Agent(agent::Command::Send { bytes, .. })) = sent.first() else {
        panic!("the shell's word back: {sent:?}");
    };
    assert_eq!(bytes, b"\x1bb");
}

#[test]
fn the_app_ring_is_heard_from_the_agent_pane_too() {
    let app = full_app();
    let mut ui = Ui::default();
    assert!(press(Key::Char('2'), ALT_SHIFT, &mut ui, &app).is_empty());
    assert_eq!(ui.session.tab, Tab::Diff);
    press(Key::Char('\''), ALT, &mut ui, &app);
    assert!(ui.session.manual && ui.focus == crate::Focus::Terminal);
}

#[test]
fn a_chord_the_config_binds_replaces_the_default() {
    let mut app = full_app();
    let mut config = bare_config();
    let rebound = [("palette.open".to_string(), vec!["alt+j".to_string()])];
    config.keymap = rebound.into_iter().collect();
    app.config.config = Some(config);
    let mut ui = Ui::default();
    press(Key::Char('k'), ALT, &mut ui, &app);
    assert!(ui.palette().is_none(), "the default gave way");
    press(Key::Char('j'), ALT, &mut ui, &app);
    assert!(ui.palette().is_some());
}

#[test]
fn a_chord_pressed_in_settings_moves_to_the_action_and_off_the_one_that_held_it() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.settings.open = true;
    ui.settings.section = crate::views::settings::Section::Keymap;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let bind = Target::SettingsBind(Action::Board);
    click(
        hits.rect_of(&bind).expect("the board's chord"),
        &mut ui,
        &app,
        &hits,
    );
    assert!(
        press(Key::Char('x'), Modifiers::default(), &mut ui, &app).is_empty(),
        "a bare key is no chord"
    );
    let asked = press(Key::Char('k'), ALT, &mut ui, &app);
    let Some(Command::Config(config::Command::Rebind(keymap))) = asked.first() else {
        panic!("{asked:?}");
    };
    let mut rebound = bare_config();
    rebound.keymap = keymap.clone();
    let keymap = Keymap::of(Some(&rebound));
    assert_eq!(keymap.chords(Action::Board), [chord("alt+k")]);
    assert!(
        keymap.chords(Action::Palette).is_empty(),
        "the palette gave its chord up"
    );
    assert!(ui.settings.binding.is_none());
}
