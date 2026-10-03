//! A run's words reaching its agent: a fresh agent asked them, or typed into the one waiting.

use groove_agent_service::runs::Run;
use groove_types::{AgentStatus, RoutineKind};

use crate::session::FIRST_SIZE;
use crate::{AppState, Spawner, agent};

/// The run's routine asked of its agent, as its kind wants.
pub(super) fn send(state: &mut AppState, spawner: &dyn Spawner, run: &Run) {
    let Some(routine) = state.agent.routine(&run.routine).cloned() else {
        return;
    };
    let words = groove_agent_service::routines::prompt(&routine, run.trigger, &run.about);
    let id = run.session.clone();
    let status = state.agent.activity(&id).map(|one| &one.status);
    let alive =
        state.agent.terminal(&id).is_some() && !matches!(status, Some(AgentStatus::Exited { .. }));
    match (routine.kind, alive) {
        (RoutineKind::Bound, true) => agent::skills::typed(state, spawner, &id, &words),
        (RoutineKind::Bound, false) => agent::asking(state, spawner, id, FIRST_SIZE, Some(words)),
        (RoutineKind::Standalone, _) => {
            agent::end(state, &id);
            if let Err(e) = groove_agent_service::fresh(&agent::launch_dir(state), id.as_str()) {
                state.failed(e);
            }
            agent::asking(state, spawner, id, FIRST_SIZE, Some(words));
        }
        (RoutineKind::Action, _) => {}
    }
}
