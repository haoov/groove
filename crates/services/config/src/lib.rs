//! The config capability. Its slice of `AppState`, the operations on it, its events.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use groove_types::{Config, Error, ThemeName, UiConfig};

/// What a font size may be, whatever the file says.
const MIN_FONT: f32 = 8.0;
const MAX_FONT: f32 = 32.0;

/// The parsed config, or nothing before first run.
#[derive(Debug, Default)]
pub struct State {
    pub config: Option<Config>,
}

/// The file under the app's config dir, read once at start.
pub fn load(config_dir: &Path) -> Result<Option<Config>, Error> {
    Ok(groove_config::load(&groove_config::path(config_dir))?)
}

impl State {
    /// The days a rule waits before it asks for the user.
    pub fn thresholds(&self) -> groove_types::Thresholds {
        self.config
            .as_ref()
            .map(|c| c.preferences.thresholds)
            .unwrap_or_default()
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

/// The config file changed under the app.
#[derive(Debug)]
pub enum Event {
    Changed,
}

pub fn apply(_state: &mut State, event: Event) {
    match event {
        Event::Changed => {}
    }
}
