//! The `config` controller: one function per user action on the `config` service.

use groove_config_service::Preference;

use crate::{AppState, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `config.set_preference`: one preference changed and the file written.
    SetPreference(Preference),
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::SetPreference(_) => "config.set_preference",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    _services: &Services,
    _spawner: &dyn Spawner,
) {
    match command {
        Command::SetPreference(one) => set_preference(state, one),
    }
}

/// The change reaches every reader at once; the file follows, written on the spot.
fn set_preference(state: &mut AppState, one: Preference) {
    let Some(config) = state.config.set(one).cloned() else {
        let e = groove_types::Error::invalid("there is no config to change before the first run");
        return state.failed(e);
    };
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        state.failed(e);
    }
}
