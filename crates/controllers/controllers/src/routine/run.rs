//! A run's words reaching its agent: a fresh agent asked them, or typed into the one waiting.

use groove_agent_service::runs::{Deliver, Run, deliver};

use crate::session::FIRST_SIZE;
use crate::{AppState, Continuation, Services, Spawner, agent};

/// The run's routine asked of its agent, as its kind wants.
pub(super) fn send(state: &mut AppState, spawner: &dyn Spawner, run: &Run) {
    let Some(routine) = state.agent.routine(&run.routine).cloned() else {
        return;
    };
    let words = groove_agent_service::routines::prompt(&routine, run.trigger, &run.about);
    let id = run.session.clone();
    match deliver(routine.kind, state.agent.alive(&id)) {
        Deliver::Type => agent::skills::typed(state, spawner, &id, &words),
        Deliver::Ask => agent::asking(state, spawner, id, FIRST_SIZE, Some(words)),
        Deliver::Fresh => {
            agent::end(state, &id);
            afresh(state, spawner, id, words);
        }
        Deliver::Nothing => {}
    }
}

/// A new conversation written for the session, then its agent asked the words.
fn afresh(state: &AppState, spawner: &dyn Spawner, id: groove_types::SessionId, words: String) {
    let dir = agent::launch_dir(state);
    spawner.spawn(Box::pin(async move {
        let written = groove_agent_service::fresh(&dir, id.as_str()).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                if let Err(e) = written {
                    state.failed(e);
                }
                agent::asking(state, spawner, id, FIRST_SIZE, Some(words));
            },
        ) as Continuation
    }));
}
