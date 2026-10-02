//! A routine switched on or off, and one trigger of it; the file written on the spot.

use groove_types::{Error, Trigger};

use crate::AppState;

/// One routine on or off; a routine no file names, or one that does not read, is not switched on.
pub(super) fn switch(state: &mut AppState, id: &str, on: bool) {
    let readable = state
        .agent
        .routines
        .iter()
        .any(|one| one.id == id && one.read.is_ok());
    if on && !readable {
        let why = format!("`{id}` is no routine Groove can run: its file is gone or does not read");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.switch_routine(id, on).cloned();
    written(state, config);
}

/// One trigger of a routine on or off.
pub(super) fn trigger(state: &mut AppState, id: &str, trigger: Trigger, on: bool) {
    let config = state.config.switch_trigger(id, trigger, on).cloned();
    written(state, config);
}

fn written(state: &mut AppState, config: Option<groove_types::Config>) {
    let Some(config) = config else {
        let e = Error::invalid("there is no config to change before the first run");
        return state.failed(e);
    };
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        state.failed(e);
    }
}
