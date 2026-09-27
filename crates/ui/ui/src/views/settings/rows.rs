//! Every row Settings shows: its section, its label, the words a search finds it by, its value.

mod appearance;
mod preferences;

use groove_controllers::AppState;
use groove_controllers::config_service::Preference;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub section: Section,
    pub label: &'static str,
    pub words: &'static str,
    pub value: Value,
}

impl Row {
    /// Whether every word of the query is in its section, label or words.
    pub fn matches(&self, query: &str) -> bool {
        let held = format!("{} {} {}", self.section.label(), self.label, self.words).to_lowercase();
        query
            .split_whitespace()
            .all(|word| held.contains(&word.to_lowercase()))
    }
}

pub fn rows(app: &AppState) -> Vec<Row> {
    let mut out = setup(app);
    out.extend(providers(app));
    out.extend(appearance::appearance(app));
    out.extend(preferences::preferences(app));
    out
}

fn text(section: Section, label: &'static str, words: &'static str, text: String) -> Row {
    let value = Value::Text { text, mono: false };
    Row {
        section,
        label,
        words,
        value,
    }
}

fn path(label: &'static str, words: &'static str, text: String) -> Row {
    let value = Value::Text { text, mono: true };
    Row {
        section: Section::Setup,
        label,
        words,
        value,
    }
}

fn setup(app: &AppState) -> Vec<Row> {
    let env = &app.env;
    let root = app.config.worktree_root(&env.home);
    vec![
        path(
            "config",
            "file path json",
            shown(&env.config_dir.join("config.json"), &env.home),
        ),
        path(
            "state",
            "database sqlite path",
            shown(&env.data_dir.join("app.db"), &env.home),
        ),
        path(
            "worktree root",
            "path pool clones git",
            shown(&root, &env.home),
        ),
    ]
}

fn providers(app: &AppState) -> Vec<Row> {
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let shown = match sources.is_empty() {
        true => "none yet".to_string(),
        false => sources
            .iter()
            .map(|one| one.label())
            .collect::<Vec<_>>()
            .join(" · "),
    };
    vec![text(
        Section::Providers,
        "task sources",
        "provider notion github tasks",
        shown,
    )]
}

/// A path under home as `~/…`.
fn shown(path: &std::path::Path, home: &std::path::Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}
