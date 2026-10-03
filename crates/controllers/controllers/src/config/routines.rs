//! A routine switched on or off, and one trigger of it; the file written on the spot.

use groove_types::{Error, Routine, RoutineKind, Timestamp, Trigger};

use crate::{AppState, Services, Spawner};

/// One routine on or off, a standalone one's session made or deleted; an unreadable one stays off.
pub(super) fn switch(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (id, on): (&str, bool),
) {
    let routine = state.agent.routine(id).cloned();
    if on && routine.is_none() {
        let why = format!("`{id}` is no routine Groove can run: its file is gone or does not read");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.switch_routine(id, on).cloned();
    super::written(state, config);
    match (on, routine) {
        (true, Some(one)) if one.kind == RoutineKind::Standalone => {
            made(state, services, spawner, &one)
        }
        (false, _) => {
            if let Some(session) = state.session.routine_session(id) {
                crate::session::delete(state, services, spawner, &session, true);
            }
        }
        _ => {}
    }
}

/// The routine's session, written and on the rail with its agent, the selection kept.
fn made(state: &mut AppState, services: &Services, spawner: &dyn Spawner, routine: &Routine) {
    if state.session.routine_session(&routine.id).is_some() {
        return;
    }
    let now = Timestamp::now();
    let session = groove_session_service::routine_session(routine, now);
    let beside = crate::session::Start {
        prompt: None,
        beside: true,
    };
    crate::session::begun_with(state, spawner, session.clone(), now, beside);
    let service = services.session.clone();
    crate::session::listed(spawner, async move {
        service.create_routine(&session, now).await
    });
}

/// One trigger of a routine on or off.
pub(super) fn trigger(state: &mut AppState, id: &str, trigger: Trigger, on: bool) {
    let config = state.config.switch_trigger(id, trigger, on).cloned();
    super::written(state, config);
}
