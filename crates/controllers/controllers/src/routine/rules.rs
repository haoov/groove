//! Which routines an event starts, and on which session: never the selected one, once a session.

use groove_agent_service::runs::{Fired, Run};
use groove_types::{Routine, RoutineKind, SessionId};

use crate::AppState;

/// Every switched-on routine that answers to the event, queued on the session it runs on.
pub(super) fn queue(state: &mut AppState, fired: &Fired) {
    let held = state.config.routines();
    let paused = state
        .config
        .config
        .as_ref()
        .is_some_and(|one| one.preferences.routines_paused);
    if paused {
        return;
    }
    let answering: Vec<Routine> = state
        .agent
        .routines
        .iter()
        .filter_map(|one| one.read.as_ref().ok())
        .filter(|one| held.on.contains(&one.id) && one.on.contains(&fired.trigger))
        .filter(|one| {
            !held
                .quiet
                .get(&one.id)
                .is_some_and(|quiet| quiet.contains(&fired.trigger))
        })
        .cloned()
        .collect();
    for routine in answering {
        let Some(session) = target(state, &routine, fired.session.as_ref()) else {
            continue;
        };
        let runs = &state.agent.runs;
        let ran = runs.ran(&routine.id, &session) || runs.holds(&routine.id, &session);
        if ran || state.session.selected.as_ref() == Some(&session) {
            continue;
        }
        state.agent.runs.queue(Run {
            routine: routine.id,
            session,
            trigger: Some(fired.trigger),
            about: fired.about.clone(),
            sent_at: None,
            went: false,
        });
    }
}

/// A bound routine runs on the open session the event is about; a standalone one in its own.
pub(super) fn target(
    state: &AppState,
    routine: &Routine,
    about: Option<&SessionId>,
) -> Option<SessionId> {
    match routine.kind {
        RoutineKind::Standalone => crate::config::session_of(state, &routine.id)
            .filter(|session| state.session.get(session).is_some()),
        RoutineKind::Bound => {
            let open = state.session.get(about?)?;
            open.session
                .kind
                .routine()
                .is_none()
                .then(|| open.session.id.clone())
        }
    }
}
