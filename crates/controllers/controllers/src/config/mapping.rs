//! What a source holds, read for the mapping; one change to it, held, written, the tasks read again.

use groove_types::{Mapping, ProviderId};

use crate::{AppState, Continuation, Services, Spawner};

/// One read of a source at a time; a failed one holds nothing until it is asked again.
pub(super) fn read(state: &mut AppState, spawner: &dyn Spawner, source: ProviderId) {
    if state.config.reading.contains(&source) {
        return;
    }
    state.config.reading.push(source);
    let sources = state.task.sources(state.config.config.as_ref());
    spawner.spawn(Box::pin(async move {
        let read = groove_task_service::schema(&sources, source).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.config.reading.retain(|one| *one != source);
            let properties = read.unwrap_or_else(|e| {
                state.failed(e);
                Vec::new()
            });
            state.config.schemas.retain(|(id, _)| *id != source);
            state.config.schemas.push((source, properties));
        }) as Continuation
    }));
}

pub(super) fn map(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (source, change): (ProviderId, Mapping),
) {
    let config = match state.config.map(source, &change) {
        Ok(config) => config.clone(),
        Err(e) => return state.failed(e),
    };
    if let Err(e) = groove_config_service::save(&state.env.config_dir, &config) {
        return state.failed(e);
    }
    crate::task::load(state, services, spawner);
}
