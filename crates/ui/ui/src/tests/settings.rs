//! Settings: opened from the rail and the palette, over the whole window, its rows searched and set.

use groove_controllers::config_service::{Font, Preference};
use groove_controllers::{AppState, Command, config};
use groove_gfx::Fonts;
use groove_types::{Found, ThemeName, Tool};

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

#[test]
fn opening_settings_checks_the_environment_again() {
    let app = full_app();
    let mut ui = Ui::default();
    let (_, hits) = drawn(&app, &ui);
    let footer = hits.rect_of(&Target::SettingsOpen).expect("the footer row");
    let asked = click(footer, &mut ui, &app, &hits);
    assert_eq!(asked, [Command::Config(config::Command::CheckEnvironment)]);
}

#[test]
fn setup_shows_each_program_in_its_state_and_offers_the_claude_sign_in() {
    let (mut app, mut ui) = opened();
    ui.settings.section = crate::views::settings::Section::Setup;
    let found = |version: &str, signed_in| {
        Some(Found {
            version: version.into(),
            signed_in,
        })
    };
    let tool = |name, required, found| Tool {
        name,
        purpose: "what it does",
        required,
        found,
    };
    app.config.tools = Some(vec![
        tool("git", true, found("2.43.0", None)),
        tool("claude", true, found("2.1.3", Some(false))),
        tool("glab", false, None),
    ]);
    let (texts, hits) = drawn(&app, &ui);
    for shown in [
        "2 not ready",
        "2.43.0",
        "2.1.3 · not signed in",
        "not found · what it does",
    ] {
        assert!(texts.iter().any(|t| t == shown), "{shown}: {texts:?}");
    }
    let sign_in = hits
        .rect_of(&Target::SettingsLogin)
        .expect("claude's sign in");
    let asked = click(sign_in, &mut ui, &app, &hits);
    let login = config::Command::Login { cols: 80, rows: 24 };
    assert_eq!(asked, [Command::Config(login)]);
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
