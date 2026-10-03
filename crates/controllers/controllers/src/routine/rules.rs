//! Which routines an event starts, and where: once a session, not the selected one, auto-approve on.

use groove_agent_service::runs::{Fired, Run};
use groove_types::{Action, Routine, RoutineKind, SessionId};

use crate::AppState;

/// Every switched-on routine that answers to the event, queued on its session; the actions to do.
pub(super) fn queue(state: &mut AppState, fired: &Fired) -> Vec<Action> {
    let held = state.config.routines();
    if state.config.preferences().routines_paused {
        return Vec::new();
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
    let mut actions = Vec::new();
    for routine in answering {
        if let Some(action) = routine.action {
            actions.push(action);
            continue;
        }
        let Some(session) = target(state, &routine, fired.session.as_ref()) else {
            continue;
        };
        let runs = &state.agent.runs;
        let ran = runs.ran(&routine.id, &session) || runs.holds(&routine.id, &session);
        if ran || !acts(state, &session) {
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
    actions
}

/// Whether an event may act on the session: not the selected one, only where auto-approve is on.
fn acts(state: &AppState, session: &SessionId) -> bool {
    let selected = state.session.selected.as_ref() == Some(session);
    let auto = state
        .session
        .get(session)
        .is_some_and(|one| one.state.auto_approve);
    auto && !selected
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
        RoutineKind::Action => None,
    }
}
