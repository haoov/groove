//! Finishing a task: the source hears it first, then the session is torn down.

use groove_types::{SessionId, SessionKind, StatusIntent, TaskKey};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner, session};

/// The task done at its source, and only then its session taken away. Work that is
/// not committed or pushed stops the teardown, and the status stands.
pub(crate) fn finish(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    asker: Asker,
) {
    let Some(open) = state.session.get(id) else {
        return asker.refused(crate::tools::NO_SESSION);
    };
    let SessionKind::Task { external_id } = &open.session.kind else {
        return asker.refused("this session works no task");
    };
    let key = match TaskKey::parse(external_id) {
        Ok(key) => key,
        Err(e) => return asker.failed(state, e),
    };
    let sources = state.task.sources(state.config.config.as_ref());
    if sources.is_empty() {
        torn_down(state, services, spawner, id);
        return asker.done(|| "no source holds it; its session goes".into());
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
                        asker.done(|| "the task is done at its source; its session goes".into());
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
