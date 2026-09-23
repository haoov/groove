//! The status Groove writes: what it did to the task, never what the user typed.

use groove_types::{ExternalId, StatusIntent, TaskKey};

use super::sync;
use crate::{AppState, Continuation, Services, Spawner};

/// The task's status at its source, then the task read again to show it.
pub(super) fn set(
    state: &mut AppState,
    spawner: &dyn Spawner,
    id: &ExternalId,
    intent: StatusIntent,
) {
    if state
        .task
        .by_external(id)
        .is_some_and(|task| task.intent == Some(intent))
    {
        return;
    }
    let key = match TaskKey::parse(id) {
        Ok(key) => key,
        Err(e) => return state.failed(e),
    };
    let sources = groove_task_service::sources(state.config.config.as_ref());
    if sources.is_empty() {
        return;
    }
    let job = state.begin(format!("setting {id}"));
    spawner.spawn(Box::pin(async move {
        let wrote = groove_task_service::set_status(&sources, &key, intent).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match wrote {
                    Ok(_) => sync(state, spawner, key),
                    Err(e) => state.failed(e),
                }
            },
        ) as Continuation
    }));
}
