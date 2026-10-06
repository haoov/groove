mod clusters;
mod sources;

pub use clusters::{ClusterChange, ClusterConfig, Hue};

pub use sources::{
    EstimateUnit, FilterConfig, GithubConfig, GithubView, NotionConfig, NotionView, PriorityMap,
    PropertyNames, REDACTED, StatusMap,
};

use std::collections::BTreeMap;

use crate::Thresholds;

/// The config file. `Config` is the on-disk shape; `ConfigView` is what the ui sees.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notion: Option<NotionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github: Option<GithubConfig>,
    pub git: GitConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub preferences: Preferences,
    /// The chords rebound: an action's id to what replaces its defaults.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub keymap: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<SharedConfig>,
    /// The user's own skills not given to sessions, by id; any other is on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills_off: Vec<String>,
    #[serde(default, skip_serializing_if = "RoutinesConfig::is_empty")]
    pub routines: RoutinesConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clusters: Vec<ClusterConfig>,
}

/// The routines switched on, and the triggers the user turned off on each.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoutinesConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub quiet: BTreeMap<String, Vec<crate::Trigger>>,
}

impl RoutinesConfig {
    pub fn is_empty(&self) -> bool {
        self.on.is_empty() && self.quiet.is_empty()
    }
}

/// The repo a team shares its skills through, and the branch Groove follows in it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SharedConfig {
    pub url: String,
    pub branch: String,
    /// The shared skills given to sessions, by id; any other is off.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
    Latte,
    Frappe,
    Macchiato,
    Mocha,
}

impl ThemeName {
    pub const ALL: [ThemeName; 4] = [
        ThemeName::Latte,
        ThemeName::Frappe,
        ThemeName::Macchiato,
        ThemeName::Mocha,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeName::Latte => "latte",
            ThemeName::Frappe => "frappé",
            ThemeName::Macchiato => "macchiato",
            ThemeName::Mocha => "mocha",
        }
    }
}

/// A vendored mono family; the file holds `stored`, empty for IBM Plex Mono.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontFamily {
    #[default]
    Plex,
    JetBrainsMono,
    Lilex,
}

impl FontFamily {
    pub const ALL: [FontFamily; 3] = [
        FontFamily::Plex,
        FontFamily::JetBrainsMono,
        FontFamily::Lilex,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FontFamily::Plex => "ibm plex",
            FontFamily::JetBrainsMono => "jetbrains mono",
            FontFamily::Lilex => "lilex",
        }
    }

    pub fn stored(self) -> &'static str {
        match self {
            FontFamily::Plex => "",
            FontFamily::JetBrainsMono => "JetBrains Mono",
            FontFamily::Lilex => "Lilex",
        }
    }

    /// A name no vendored family stores reads as IBM Plex Mono.
    pub fn named(name: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|one| one.stored() == name)
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UiConfig {
    /// The interface's type size; code has its own.
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_code_font_size")]
    pub code_font_size: f32,
    /// Unset means the code size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_font_size: Option<f32>,
    #[serde(default)]
    pub theme: ThemeName,
    /// Empty means the bundled font.
    #[serde(default)]
    pub font_family: String,
    /// Code, the agent and the terminals.
    #[serde(default, alias = "agent_font_family")]
    pub mono_font_family: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            code_font_size: default_code_font_size(),
            terminal_font_size: None,
            theme: ThemeName::default(),
            font_family: String::new(),
            mono_font_family: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitConfig {
    /// The pool lives under `<worktree_root>/main`, the worktrees under `<worktree_root>/worktrees`.
    pub worktree_root: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub auto_approve_default: bool,
    #[serde(default)]
    pub thresholds: Thresholds,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_stale_after")]
    pub stale_after_secs: u64,
    /// How many agents run routines at once; a run past it waits its turn.
    #[serde(default = "default_routine_cap")]
    pub routine_cap: u32,
    /// Every routine paused, whatever each one is switched to.
    #[serde(default)]
    pub routines_paused: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_approve_default: false,
            thresholds: Thresholds::default(),
            poll_interval_secs: default_poll_interval(),
            stale_after_secs: default_stale_after(),
            routine_cap: default_routine_cap(),
            routines_paused: false,
        }
    }
}

fn default_routine_cap() -> u32 {
    5
}

/// The config as the ui sees it: everything except the Notion token.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ConfigView {
    pub notion: Option<NotionView>,
    pub github: Option<GithubView>,
    pub git: GitConfig,
    pub ui: UiConfig,
    pub preferences: Preferences,
}

impl From<Config> for ConfigView {
    fn from(c: Config) -> Self {
        Self {
            notion: c.notion.map(NotionView::from),
            github: c.github.map(GithubView::from),
            git: c.git,
            ui: c.ui,
            preferences: c.preferences,
        }
    }
}

fn default_font_size() -> f32 {
    13.0
}

fn default_code_font_size() -> f32 {
    12.5
}

fn default_poll_interval() -> u64 {
    60
}

fn default_stale_after() -> u64 {
    300
}
