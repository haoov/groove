//! Finishing a task: the source hears it first, then the session is torn down.

use groove_types::{SessionId, SessionKind, StatusIntent, TaskKey};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner, session};

/// What the agent hears before its own session ends: the teardown's outcome comes after it.
const TEARDOWN: &str = "its session is taken away now, unless it holds work not yet pushed, \
which Groove then shows and keeps";

/// The task done at its source, then its session taken away; lost work stops the teardown.
pub(crate) fn finish(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    asker: Asker,
) {
    let Some(open) = state.session.get(id) else {
        return asker.refused(state, crate::tools::NO_SESSION);
    };
    let SessionKind::Task { external_id } = &open.session.kind else {
        return asker.refused(state, super::NOT_A_TASK);
    };
    let key = match TaskKey::parse(external_id) {
        Ok(key) => key,
        Err(e) => return asker.failed(state, e),
    };
    let sources = state.task.sources(state.config.config.as_ref());
    if sources.is_empty() {
        asker.done(|| format!("no source holds it; {TEARDOWN}"));
        return torn_down(state, services, spawner, id);
    }
    let job = state.begin(format!("finishing {}", open.session.title));
    let id = id.clone();
    spawner.spawn(Box::pin(async move {
        let wrote = groove_task_service::set_status(&sources, &key, StatusIntent::Done).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match wrote {
                    Ok(_) => {
                        asker.done(|| format!("the task is done at its source; {TEARDOWN}"));
                        torn_down(state, services, spawner, &id);
                    }
                    Err(e) => asker.failed(state, e),
                }
            },
        ) as Continuation
    }));
}

/// The session gone, its worktrees with it, and the tasks read again.
fn torn_down(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    session::delete(state, services, spawner, id, false);
    super::load(state, services, spawner);
}
