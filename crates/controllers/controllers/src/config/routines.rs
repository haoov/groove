//! A routine switched on or off, and one trigger of it; the file written on the spot.

use groove_types::{Error, Routine, RoutineKind, SessionId, Timestamp, Trigger};

use crate::{AppState, Services, Spawner};

/// One routine on or off, a standalone one's session made or deleted; an unreadable one stays off.
pub(super) fn switch(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (id, on): (&str, bool),
) {
    let read = state.agent.routines.iter().find(|one| one.id == id);
    let routine = read.and_then(|one| one.read.as_ref().ok()).cloned();
    if on && routine.is_none() {
        let why = format!("`{id}` is no routine Groove can run: its file is gone or does not read");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.switch_routine(id, on).cloned();
    written(state, config);
    match (on, routine) {
        (true, Some(one)) if one.kind == RoutineKind::Standalone => {
            made(state, services, spawner, &one)
        }
        (false, _) => {
            if let Some(session) = session_of(state, id) {
                crate::session::delete(state, services, spawner, &session, true);
            }
        }
        _ => {}
    }
}

/// The routine's session, written and on the rail with its agent, the selection kept.
fn made(state: &mut AppState, services: &Services, spawner: &dyn Spawner, routine: &Routine) {
    if session_of(state, &routine.id).is_some() {
        return;
    }
    let now = Timestamp::now();
    let session = groove_session_service::routine_session(routine, now);
    let selected = state.session.selected.clone();
    crate::session::begun(state, spawner, session.clone(), now);
    state.session.selected = selected;
    crate::workspace::follow(state, spawner);
    let service = services.session.clone();
    crate::session::listed(spawner, async move {
        service.create_routine(&session, now).await
    });
}

/// The session a routine runs in, among every session on disk and on the rail.
pub(crate) fn session_of(state: &AppState, routine: &str) -> Option<SessionId> {
    let on_rail = state.session.open.iter().map(|one| &one.session);
    let on_disk = state.session.living.iter().map(|one| &one.session);
    on_rail
        .chain(on_disk)
        .find(|one| one.kind.routine() == Some(routine))
        .map(|one| one.id.clone())
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
