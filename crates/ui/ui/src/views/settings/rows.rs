//! Every row Settings shows: its section, its label, the words a search finds it by, its value.

mod agent;
mod appearance;
mod drafting;
mod keymap;
mod mapping;
mod preferences;
mod providers;
mod routines;
mod setup;
mod shared;
mod skills;
mod switch;

pub use mapping::{Choices, Slot, choices};

use groove_controllers::AppState;
use groove_controllers::config_service::Preference;
use groove_ui_kit::base::style::Role;

use crate::hit::Target;

/// The sections, in the order the list stands.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Setup,
    Providers,
    Agent,
    Appearance,
    #[default]
    Preferences,
    Keymap,
}

impl Section {
    pub const ALL: [Section; 6] = [
        Section::Setup,
        Section::Providers,
        Section::Agent,
        Section::Appearance,
        Section::Preferences,
        Section::Keymap,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::Setup => "Setup",
            Section::Providers => "Providers",
            Section::Agent => "Agent",
            Section::Appearance => "Appearance",
            Section::Preferences => "Preferences",
            Section::Keymap => "Keymap",
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
    /// A value as the button that changes it, and what a click beside it does.
    Picker {
        shown: String,
        role: Role,
        target: Target,
        act: Option<(&'static str, Target)>,
    },
    /// A field being typed into, which a click gives the keys.
    Input {
        shown: String,
        focused: bool,
        target: Target,
    },
    /// A skill: on, off, or `None` for always on; what it does; its delete, and that delete's question.
    Skill {
        id: String,
        on: Option<bool>,
        said: String,
        deletes: bool,
        asking: bool,
    },
    /// A routine: on or off and what it does; an action routine's run button while on.
    Routine {
        id: String,
        on: bool,
        said: String,
        runs: bool,
    },
    /// A word on or off, and what a click on it sets.
    Switch {
        on: bool,
        target: Target,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub section: Section,
    /// The heading it stands under within its section.
    pub group: &'static str,
    pub label: std::borrow::Cow<'static, str>,
    pub words: &'static str,
    pub value: Value,
}

impl Row {
    /// One row in `section` under no heading of its own; `grouped` gives it one.
    pub fn new(
        section: Section,
        label: impl Into<std::borrow::Cow<'static, str>>,
        words: &'static str,
        value: Value,
    ) -> Self {
        Self {
            section,
            group: "",
            label: label.into(),
            words,
            value,
        }
    }

    /// Whether every word of the query is in its section, group, label or words.
    pub fn matches(&self, query: &str) -> bool {
        let (section, group) = (self.section.label(), self.group);
        let held = format!("{section} {group} {} {}", self.label, self.words).to_lowercase();
        query
            .split_whitespace()
            .all(|word| held.contains(&word.to_lowercase()))
    }

    /// A row under the one before it, in its block: a routine's trigger.
    pub fn sub(&self) -> bool {
        matches!(self.value, Value::Switch { .. })
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
    out.extend(agent::agent(app, settings));
    out.extend(appearance::appearance(app));
    out.extend(preferences::preferences(app));
    out.extend(keymap::keymap(app, settings));
    out
}

fn text(section: Section, label: &'static str, words: &'static str, text: String) -> Row {
    let value = Value::Text { text, mono: false };
    Row::new(section, label, words, value)
}
