//! A routine switched on or off, and one trigger of it; the file follows.

use groove_types::{Error, Trigger};

use crate::{AppState, Services, Spawner};

/// One routine on or off; an unreadable one stays off.
pub(super) fn switch(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (id, on): (&str, bool),
) {
    if on && state.routine.routine(id).is_none() {
        let why = format!("`{id}` is no routine Groove can run: its file is gone or does not read");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.switch_routine(id, on).cloned();
    super::written(state, services, spawner, config);
}

/// One trigger of a routine on or off.
pub(super) fn trigger(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (id, trigger, on): (&str, Trigger, bool),
) {
    let config = state.config.switch_trigger(id, trigger, on).cloned();
    super::written(state, services, spawner, config);
}
