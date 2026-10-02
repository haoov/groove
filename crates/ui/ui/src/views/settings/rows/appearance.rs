//! The Appearance section's rows: the theme to pick, the families, a size per font.

use groove_controllers::AppState;
use groove_controllers::config_service::{Font, MIN_FONT, Preference};
use groove_types::ThemeName;

use super::preferences::count;
use super::{Row, Section, Value, grouped, text};

pub(super) fn appearance(app: &AppState) -> Vec<Row> {
    let ui = app
        .config
        .config
        .as_ref()
        .map(|c| c.ui.clone())
        .unwrap_or_default();
    let family = |name: &str| match name.is_empty() {
        true => "bundled".to_string(),
        false => name.to_string(),
    };
    let section = Section::Appearance;
    let themes = ThemeName::ALL.map(|one| (one.label(), one == ui.theme, Preference::Theme(one)));
    let theme = Row {
        section,
        group: "",
        label: "theme".into(),
        words: "latte frappe macchiato mocha colour dark light",
        value: Value::Choice(themes.to_vec()),
    };
    let mut fonts = vec![
        text(section, "ui font", "family type", family(&ui.font_family)),
        text(
            section,
            "agent font",
            "family terminal mono",
            family(&ui.agent_font_family),
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
