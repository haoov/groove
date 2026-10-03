//! The config capability: the parsed config file, the preferences and the last environment check.

mod sources;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use sources::Source;

use groove_types::{Config, Error, Preferences, ThemeName, Tool, UiConfig};

/// What a font size may be, whatever the file says.
pub const MIN_FONT: f32 = 8.0;
const MAX_FONT: f32 = 32.0;

/// The shortest wait between two polls of the forges.
const POLL_MIN: u64 = 10;

/// One preference, as Settings changes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Preference {
    Theme(ThemeName),
    /// In points; the file keeps it inside what a font can read as.
    FontSize(Font, f32),
    AutoApproveDefault(bool),
    ReviewWaitingDays(u32),
    DueSoonDays(u32),
    ApprovedUnmergedDays(u32),
    CiFailedMinutes(u32),
    PollIntervalSecs(u64),
    StaleAfterSecs(u64),
    RoutineCap(u32),
    RoutinesPaused(bool),
}

/// A size is never NaN: it comes from the file's number or a step of it.
impl Eq for Preference {}

/// What one font size draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Interface,
    Editor,
    Terminal,
}

/// The parsed config, or nothing before first run.
#[derive(Debug, Default)]
pub struct State {
    pub config: Option<Config>,
    /// What the last environment check found; `None` before the first one ends.
    pub tools: Option<Vec<Tool>>,
    pub checking: bool,
    /// The source a connect is reading, and why the last one was refused.
    pub connecting: Option<groove_types::ProviderId>,
    pub refused: Option<String>,
    /// A shared repo is being copied, and why the last one was refused.
    pub joining: bool,
    pub unshared: Option<String>,
    /// What each source was last read to hold, and the sources being read.
    pub schemas: Vec<(groove_types::ProviderId, Vec<groove_types::Property>)>,
    pub reading: Vec<groove_types::ProviderId>,
}

/// Every program the app runs, checked; `claude` is the path the agent launches.
pub async fn check(claude: &str) -> Vec<Tool> {
    groove_config::check(claude).await
}

/// The file under the app's config dir, read once at start.
pub fn load(config_dir: &Path) -> Result<Option<Config>, Error> {
    Ok(groove_config::load(&groove_config::path(config_dir))?)
}

/// `<config dir>/config.json`.
pub fn path(config_dir: &Path) -> std::path::PathBuf {
    groove_config::path(config_dir)
}

/// A change made to the config, written to its file; there is none to change before the first run.
pub fn keep(config_dir: &Path, changed: Option<&Config>) -> Result<(), Error> {
    let none = || Error::invalid("there is no config to change before the first run");
    save(config_dir, changed.ok_or_else(none)?)
}

pub fn save(config_dir: &Path, config: &Config) -> Result<(), Error> {
    Ok(groove_config::save(
        &groove_config::path(config_dir),
        config,
    )?)
}

impl State {
    pub fn preferences(&self) -> Preferences {
        let held = self.config.as_ref().map(|c| c.preferences.clone());
        held.unwrap_or_default()
    }

    /// The days a rule waits before it asks for the user.
    pub fn thresholds(&self) -> groove_types::Thresholds {
        self.preferences().thresholds
    }

    /// How long the poll waits between two passes over the open MRs, in seconds.
    pub fn poll_interval(&self) -> i64 {
        self.preferences().poll_interval_secs.max(POLL_MIN) as i64
    }

    /// How old an MR's last read may be before its row says so, in seconds.
    pub fn stale_after(&self) -> i64 {
        self.preferences().stale_after_secs as i64
    }

    /// Whether a new session's writes run without asking.
    pub fn auto_approve_default(&self) -> bool {
        self.preferences().auto_approve_default
    }

    /// The shared repo named, or none; the config to write, or `None` before first run.
    pub fn set_shared(&mut self, shared: Option<groove_types::SharedConfig>) -> Option<&Config> {
        let config = self.config.as_mut()?;
        config.shared = shared;
        Some(config)
    }

    pub fn shared(&self) -> Option<&groove_types::SharedConfig> {
        self.config.as_ref()?.shared.as_ref()
    }

    pub fn routines(&self) -> groove_types::RoutinesConfig {
        self.config
            .as_ref()
            .map(|one| one.routines.clone())
            .unwrap_or_default()
    }

    /// One routine switched on or off; switched off, the triggers it had turned off are forgotten.
    pub fn switch_routine(&mut self, id: &str, on: bool) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.routines;
        held.on.retain(|one| one != id);
        match on {
            true => held.on.push(id.to_string()),
            false => drop(held.quiet.remove(id)),
        }
        self.config.as_ref()
    }

    /// One trigger of a routine turned on or off.
    pub fn switch_trigger(
        &mut self,
        id: &str,
        trigger: groove_types::Trigger,
        on: bool,
    ) -> Option<&Config> {
        let held = &mut self.config.as_mut()?.routines;
        let quiet = held.quiet.entry(id.to_string()).or_default();
        quiet.retain(|one| *one != trigger);
        if !on {
            quiet.push(trigger);
        }
        if quiet.is_empty() {
            held.quiet.remove(id);
        }
        self.config.as_ref()
    }

    pub fn skills_off(&self) -> &[String] {
        self.config
            .as_ref()
            .map_or(&[], |one| one.skills_off.as_slice())
    }

    /// One skill switched on or off: a user's skill in `skills_off`, a shared one in `enabled`.
    pub fn switch_skill(&mut self, id: &str, on: bool) -> Option<&Config> {
        let config = self.config.as_mut()?;
        let (held, wanted) = match id.starts_with("user:") {
            true => (&mut config.skills_off, !on),
            false => (&mut config.shared.as_mut()?.enabled, on),
        };
        held.retain(|one| one != id);
        if wanted {
            held.push(id.to_string());
        }
        Some(config)
    }

    /// The chords rebound, whole; the config to write, or `None` before first run.
    pub fn rebind(&mut self, keymap: BTreeMap<String, Vec<String>>) -> Option<&Config> {
        let config = self.config.as_mut()?;
        config.keymap = keymap;
        Some(config)
    }

    /// One preference changed; the config to write, or `None` before first run.
    pub fn set(&mut self, one: Preference) -> Option<&Config> {
        let config = self.config.as_mut()?;
        let (ui, held) = (&mut config.ui, &mut config.preferences);
        match one {
            Preference::Theme(theme) => ui.theme = theme,
            Preference::FontSize(font, size) => {
                let size = size.clamp(MIN_FONT, MAX_FONT);
                match font {
                    Font::Interface => ui.font_size = size,
                    Font::Editor => ui.code_font_size = size,
                    Font::Terminal => ui.terminal_font_size = Some(size),
                }
            }
            Preference::AutoApproveDefault(on) => held.auto_approve_default = on,
            Preference::ReviewWaitingDays(days) => held.thresholds.review_waiting_days = days,
            Preference::DueSoonDays(days) => held.thresholds.due_soon_days = days,
            Preference::ApprovedUnmergedDays(days) => held.thresholds.approved_unmerged_days = days,
            Preference::CiFailedMinutes(minutes) => held.thresholds.ci_failed_minutes = minutes,
            Preference::PollIntervalSecs(secs) => held.poll_interval_secs = secs.max(POLL_MIN),
            Preference::StaleAfterSecs(secs) => held.stale_after_secs = secs,
            Preference::RoutineCap(agents) => held.routine_cap = agents.max(1),
            Preference::RoutinesPaused(paused) => held.routines_paused = paused,
        }
        Some(config)
    }

    pub fn theme(&self) -> ThemeName {
        self.config.as_ref().map(|c| c.ui.theme).unwrap_or_default()
    }

    /// The interface's type size, code's and the terminals', each inside what a font can read as.
    pub fn text_size(&self) -> f32 {
        self.sized(|ui| ui.font_size)
    }

    pub fn code_size(&self) -> f32 {
        self.sized(|ui| ui.code_font_size)
    }

    pub fn terminal_size(&self) -> f32 {
        self.sized(|ui| ui.terminal_font_size.unwrap_or(ui.code_font_size))
    }

    pub fn size(&self, font: Font) -> f32 {
        match font {
            Font::Interface => self.text_size(),
            Font::Editor => self.code_size(),
            Font::Terminal => self.terminal_size(),
        }
    }

    fn sized(&self, of: impl Fn(&UiConfig) -> f32) -> f32 {
        let default = of(&UiConfig::default());
        self.config
            .as_ref()
            .map(|c| of(&c.ui))
            .filter(|size| *size >= MIN_FONT && *size <= MAX_FONT)
            .unwrap_or(default)
    }

    /// Where every agent runs: `git.worktree_root`, `~` expanded; `home` before first run.
    pub fn worktree_root(&self, home: &Path) -> PathBuf {
        match self.config.as_ref().map(|c| c.git.worktree_root.as_str()) {
            Some(root) if !root.is_empty() => expand_tilde(root, home),
            _ => home.to_path_buf(),
        }
    }
}

fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None if path == "~" => home.to_path_buf(),
        None => PathBuf::from(path),
    }
}

/// What the outside world tells this capability.
#[derive(Debug)]
pub enum Event {
    Checked(Vec<Tool>),
}

pub fn apply(state: &mut State, event: Event) {
    match event {
        Event::Checked(tools) => {
            state.tools = Some(tools);
            state.checking = false;
        }
    }
}
