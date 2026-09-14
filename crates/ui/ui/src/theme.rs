use groove_controllers::AppState;
use groove_gfx::Theme;
use groove_types::ThemeName;

/// The theme the config names; Latte before first run.
pub fn theme(state: &AppState) -> Theme {
    match state.config.theme() {
        ThemeName::Latte => Theme::light(),
        ThemeName::Frappe => Theme::frappe(),
        ThemeName::Macchiato => Theme::macchiato(),
        ThemeName::Mocha => Theme::dark(),
    }
}
