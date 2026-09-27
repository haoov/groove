//! The config capability. Its slice of `AppState`, the operations on it, its events.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use groove_types::{Config, Error, Preferences, ThemeName, UiConfig};

/// What a font size may be, whatever the file says.
const MIN_FONT: f32 = 8.0;
const MAX_FONT: f32 = 32.0;

/// The shortest wait between two polls of the forges.
const POLL_MIN: u64 = 10;

/// One preference, as Settings changes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preference {
    AutoApproveDefault(bool),
    ReviewWaitingDays(u32),
    DueSoonDays(u32),
    ApprovedUnmergedDays(u32),
    CiFailedMinutes(u32),
    PollIntervalSecs(u64),
    StaleAfterSecs(u64),
}

/// The parsed config, or nothing before first run.
#[derive(Debug, Default)]
pub struct State {
    pub config: Option<Config>,
}

/// The file under the app's config dir, read once at start.
pub fn load(config_dir: &Path) -> Result<Option<Config>, Error> {
    Ok(groove_config::load(&groove_config::path(config_dir))?)
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

    /// One preference changed; the config to write, or `None` before first run.
    pub fn set(&mut self, one: Preference) -> Option<&Config> {
        let config = self.config.as_mut()?;
        let held = &mut config.preferences;
        match one {
            Preference::AutoApproveDefault(on) => held.auto_approve_default = on,
            Preference::ReviewWaitingDays(days) => held.thresholds.review_waiting_days = days,
            Preference::DueSoonDays(days) => held.thresholds.due_soon_days = days,
            Preference::ApprovedUnmergedDays(days) => held.thresholds.approved_unmerged_days = days,
            Preference::CiFailedMinutes(minutes) => held.thresholds.ci_failed_minutes = minutes,
            Preference::PollIntervalSecs(secs) => held.poll_interval_secs = secs.max(POLL_MIN),
            Preference::StaleAfterSecs(secs) => held.stale_after_secs = secs,
        }
        Some(config)
    }

    pub fn theme(&self) -> ThemeName {
        self.config.as_ref().map(|c| c.ui.theme).unwrap_or_default()
    }

    /// The interface's type size, and code's, both inside what a font can read as.
    pub fn text_size(&self) -> f32 {
        self.sized(|ui| ui.font_size)
    }

    pub fn code_size(&self) -> f32 {
        self.sized(|ui| ui.code_font_size)
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
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
