//! The config capability. Its slice of `AppState`, the operations on it, its events.

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
