//! A routine switched on or off, and one trigger of it; the file follows.

use groove_types::{Error, Routine, RoutineKind, Timestamp, Trigger};

use crate::{AppState, Services, Spawner};

/// One routine on or off, a standalone one's session made or deleted; an unreadable one stays off.
pub(super) fn switch(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (id, on): (&str, bool),
) {
    let routine = state.routine.routine(id).cloned();
    if on && routine.is_none() {
        let why = format!("`{id}` is no routine Groove can run: its file is gone or does not read");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.switch_routine(id, on).cloned();
    super::written(state, services, spawner, config);
    match (on, routine) {
        (true, Some(one)) if one.kind == RoutineKind::Standalone => {
            made(state, services, spawner, &one)
        }
        (false, _) => {
            if let Some(session) = state.session.routine_session(id) {
                let id = id.to_string();
                let then = Box::new(
                    move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                        remade(state, services, spawner, &id)
                    },
                );
                crate::session::delete_then(state, (services, spawner), (&session, true), then);
            }
        }
        _ => {}
    }
}

/// The routine's session, written and on the rail with its agent, the selection kept.
fn made(state: &mut AppState, services: &Services, spawner: &dyn Spawner, routine: &Routine) {
    if state.session.routine_session(&routine.id).is_none() {
        begun(state, services, spawner, routine);
    }
}

fn begun(state: &mut AppState, services: &Services, spawner: &dyn Spawner, routine: &Routine) {
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

/// A standalone routine switched on again while its session was being deleted, given a fresh one.
fn remade(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &str) {
    let on = state.config.routines().on.iter().any(|one| one == id);
    let routine = state.routine.routine(id).cloned();
    if let Some(one) = routine.filter(|one| on && one.kind == RoutineKind::Standalone) {
        begun(state, services, spawner, &one);
    }
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
