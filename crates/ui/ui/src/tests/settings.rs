//! Settings: opened from the rail and the palette, over the whole window, its rows searched and set.

use groove_controllers::config_service::Preference;
use groove_controllers::{AppState, Command, config};
use groove_gfx::Fonts;

use super::*;
use crate::hit::Target;
use crate::view;

fn drawn(app: &AppState, ui: &Ui) -> (Vec<String>, Hits) {
    let (frame, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let texts = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    (texts, hits)
}

fn opened() -> (AppState, Ui) {
    let app = full_app();
    let mut ui = Ui::default();
    ui.settings.open = true;
    (app, ui)
}

#[test]
fn the_rail_s_footer_opens_settings_over_the_whole_window_and_esc_comes_back() {
    let app = full_app();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let footer = hits.rect_of(&Target::SettingsOpen).expect("the footer row");
    click(footer, &mut ui, &app, &hits);
    assert!(ui.settings.open);
    let (texts, hits) = drawn(&app, &ui);
    assert!(
        hits.rect_of(&Target::Board).is_none(),
        "the rail is not drawn"
    );
    assert!(texts.iter().any(|one| one == "Preferences"), "{texts:?}");
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(!ui.settings.open);
}

#[test]
fn a_search_finds_rows_of_every_section_and_names_each() {
    let (app, mut ui) = opened();
    ui.settings.search.set("path");
    let (texts, _) = drawn(&app, &ui);
    for row in ["config", "state", "worktree root"] {
        assert!(texts.iter().any(|one| one == row), "{row}: {texts:?}");
    }
    assert!(
        texts.iter().any(|one| one == "Setup"),
        "the section beside each"
    );
}

#[test]
fn a_step_up_asks_for_the_preference_one_step_more() {
    let (app, mut ui) = opened();
    let (_, hits) = drawn(&app, &ui);
    let at = app.config.preferences().poll_interval_secs;
    let more = Target::SetPreference(Preference::PollIntervalSecs(at + 10));
    let plus = hits.rect_of(&more).expect("the poll interval's +");
    let asked = click(plus, &mut ui, &app, &hits);
    let set = config::Command::SetPreference(Preference::PollIntervalSecs(at + 10));
    assert_eq!(asked, [Command::Config(set)]);
}

#[test]
fn the_palette_opens_settings_too() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('p'), CHORD, &mut ui, &app);
    for c in "settings".chars() {
        press(Key::Char(c), Modifiers::default(), &mut ui, &app);
    }
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(ui.settings.open);
    assert!(ui.palette().is_none());
}
