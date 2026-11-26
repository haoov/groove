//! A run's words reaching its agent: asked of a new agent, or typed into the one waiting.

use groove_routine_service::runs::{Deliver, Run, deliver};

use crate::{AppState, Spawner, agent};

/// The run's routine asked of its agent, as its kind wants.
pub(super) fn send(state: &mut AppState, spawner: &dyn Spawner, run: &Run) {
    let Some(routine) = state.routine.routine(&run.routine).cloned() else {
        return;
    };
    let words = groove_routine_service::routines::prompt(&routine, run.trigger, &run.about);
    let id = run.session.clone();
    match deliver(routine.kind, state.agent.alive(&id)) {
        Deliver::Type => agent::skills::typed(state, &id, &words),
        Deliver::Ask => agent::asking(state, spawner, id, Some(words)),
        Deliver::Nothing => {}
    }
}
