//! The config capability. Its slice of `AppState`, the operations on it, its events.

use std::path::{Path, PathBuf};

use groove_types::{Config, ThemeName};

/// The parsed config, or nothing before first run.
#[derive(Debug, Default)]
pub struct State {
    pub config: Option<Config>,
}

impl State {
    pub fn theme(&self) -> ThemeName {
        self.config.as_ref().map(|c| c.ui.theme).unwrap_or_default()
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
