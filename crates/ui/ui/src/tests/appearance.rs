//! Settings › Appearance: the theme, the two font families and the three sizes.

use groove_controllers::config_service::{Font, Preference};
use groove_controllers::{Command, config};
use groove_types::{FontFamily, ThemeName};

use super::settings::{drawn, opened};
use super::*;
use crate::hit::Target;

#[test]
fn appearance_offers_every_theme_and_a_click_picks_one() {
    let (app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Appearance;
    let (texts, hits) = drawn(&app, &ui);
    for one in ThemeName::ALL {
        assert!(texts.iter().any(|t| t == one.label()), "{one:?}: {texts:?}");
    }
    let mocha = Target::SetPreference(Preference::Theme(ThemeName::Mocha));
    let at = hits.rect_of(&mocha).expect("mocha's word");
    let asked = click(at, &mut ui, &app, &hits);
    let set = config::Command::SetPreference(Preference::Theme(ThemeName::Mocha));
    assert_eq!(asked, [Command::Config(set)]);
}

#[test]
fn appearance_offers_every_family_and_a_click_picks_one() {
    let (app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Appearance;
    let (texts, hits) = drawn(&app, &ui);
    for one in FontFamily::MONO {
        let shown = texts.iter().filter(|t| *t == one.label()).count();
        let rows = 1 + usize::from(FontFamily::UI.contains(&one));
        assert_eq!(shown, rows, "{one:?} on the ui and mono rows: {texts:?}");
    }
    let jetbrains = Preference::UiFamily(FontFamily::JetBrainsMono);
    let at = hits
        .rect_of(&Target::SetPreference(jetbrains))
        .expect("jetbrains mono's word on the ui font");
    let asked = click(at, &mut ui, &app, &hits);
    let set = config::Command::SetPreference(jetbrains);
    assert_eq!(asked, [Command::Config(set)]);
}

#[test]
fn each_font_has_its_own_size_and_a_step_up_asks_for_one_point_more() {
    let (app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Appearance;
    let (texts, hits) = drawn(&app, &ui);
    for row in ["ui size", "editor size", "terminal size"] {
        assert!(texts.iter().any(|t| t == row), "{row}: {texts:?}");
    }
    let at = app.config.terminal_size();
    let more = Preference::FontSize(Font::Terminal, at + 1.0);
    let plus = hits
        .rect_of(&Target::SetPreference(more))
        .expect("the terminal's +");
    let asked = click(plus, &mut ui, &app, &hits);
    assert_eq!(
        asked,
        [Command::Config(config::Command::SetPreference(more))]
    );
}
