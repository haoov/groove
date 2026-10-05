//! The Appearance section's rows: the theme to pick, the families, a size per font.

use groove_controllers::AppState;
use groove_controllers::config_service::{Font, MIN_FONT, Preference};
use groove_types::{FontFamily, ThemeName};

use super::preferences::count;
use super::{Row, Section, Value, grouped};

pub(super) fn appearance(app: &AppState) -> Vec<Row> {
    let ui = app
        .config
        .config
        .as_ref()
        .map(|c| c.ui.clone())
        .unwrap_or_default();
    let section = Section::Appearance;
    let themes = ThemeName::ALL.map(|one| (one.label(), one == ui.theme, Preference::Theme(one)));
    let words = "latte frappe macchiato mocha colour dark light";
    let theme = Row::new(section, "theme", words, Value::Choice(themes.to_vec()));
    let families = |among: &[FontFamily], held, set: fn(FontFamily) -> Preference| {
        let one = |family: &FontFamily| (family.label(), *family == held, set(*family));
        Value::Choice(among.iter().map(one).collect())
    };
    let mut fonts = vec![
        Row::new(
            section,
            "ui font",
            "family type plex jetbrains",
            families(
                &FontFamily::UI,
                app.config.ui_family(),
                Preference::UiFamily,
            ),
        ),
        Row::new(
            section,
            "mono font",
            "family code diff agent terminal shell plex jetbrains lilex",
            families(
                &FontFamily::MONO,
                app.config.mono_family(),
                Preference::MonoFamily,
            ),
        ),
    ];
    fonts.extend(sizes(app));
    let mut out = grouped("Theme", vec![theme]);
    out.extend(grouped("Fonts", fonts));
    out
}

fn sizes(app: &AppState) -> Vec<Row> {
    let fonts = [
        (Font::Interface, "ui size", "font text points"),
        (Font::Editor, "editor size", "font code diff points"),
        (Font::Terminal, "terminal size", "font agent shell points"),
    ];
    let row = |(font, label, words)| {
        let at = app.config.size(font);
        let set = move |size| Preference::FontSize(font, size);
        Row {
            section: Section::Appearance,
            ..count(label, words, (at, 1.0, MIN_FONT), "pt", set)
        }
    };
    fonts.into_iter().map(row).collect()
}
