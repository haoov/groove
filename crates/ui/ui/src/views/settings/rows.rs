//! Every row Settings shows: its section, its label, the words a search finds it by, its value.

mod appearance;
mod preferences;
mod providers;
mod setup;
mod switch;

use groove_controllers::AppState;
use groove_controllers::config_service::Preference;
use groove_ui_kit::base::style::Role;

use crate::hit::Target;

/// The sections, in the order the list stands.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Setup,
    Providers,
    Appearance,
    #[default]
    Preferences,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Setup,
        Section::Providers,
        Section::Appearance,
        Section::Preferences,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::Setup => "Setup",
            Section::Providers => "Providers",
            Section::Appearance => "Appearance",
            Section::Preferences => "Preferences",
        }
    }
}

/// What a row holds, and what a click on it sets.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Read only here; a path or an id is `mono`.
    Text {
        text: String,
        mono: bool,
    },
    Toggle {
        on: bool,
        flip: Preference,
    },
    /// A number, with what one step down and one step up set.
    Count {
        shown: String,
        less: Option<Preference>,
        more: Preference,
    },
    /// One of a few, each with the preference that picks it; `true` is the one held.
    Choice(Vec<(&'static str, bool, Preference)>),
    /// A state in its colour, with what a click beside it does.
    State {
        shown: String,
        role: Role,
        act: Option<(&'static str, Target)>,
    },
    /// A field being typed into, which a click gives the keys.
    Input {
        shown: String,
        focused: bool,
        target: Target,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub section: Section,
    /// The heading it stands under within its section.
    pub group: &'static str,
    pub label: &'static str,
    pub words: &'static str,
    pub value: Value,
}

impl Row {
    /// Whether every word of the query is in its section, group, label or words.
    pub fn matches(&self, query: &str) -> bool {
        let (section, group) = (self.section.label(), self.group);
        let held = format!("{section} {group} {} {}", self.label, self.words).to_lowercase();
        query
            .split_whitespace()
            .all(|word| held.contains(&word.to_lowercase()))
    }
}

/// Every row of `rows` under one heading.
fn grouped(group: &'static str, rows: Vec<Row>) -> Vec<Row> {
    let under = |row| Row { group, ..row };
    rows.into_iter().map(under).collect()
}

pub fn rows(app: &AppState, settings: &super::SettingsUi) -> Vec<Row> {
    let mut out = setup::setup(app);
    out.extend(providers::providers(app, settings));
    out.extend(appearance::appearance(app));
    out.extend(preferences::preferences(app));
    out
}

fn text(section: Section, label: &'static str, words: &'static str, text: String) -> Row {
    let value = Value::Text { text, mono: false };
    Row {
        section,
        group: "",
        label,
        words,
        value,
    }
}
