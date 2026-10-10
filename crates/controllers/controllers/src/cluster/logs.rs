//! The log streams the resource tabs read: opened with their first reader, closed after the last.

use groove_cluster_service::Event as ClusterEvent;
use groove_types::LogKey;

use super::objects::{IDLE, known};
use crate::{AppState, Event, Services, Spawner, apply};

pub(super) fn follow(state: &mut AppState, spawner: &dyn Spawner, reader: &str, key: LogKey) {
    if !known(state, &key.context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    let Some(stop) = state.cluster.store.logs.lease(&key, reader) else {
        return;
    };
    let (sink, held) = (spawner.sink(), key.clone());
    let send = move |batch| {
        let key = Box::new(held.clone());
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                apply(Event::Cluster(ClusterEvent::Logged { key, batch }), state)
            },
        ));
    };
    let logger = groove_cluster_service::logger(paths, key, stop, send);
    spawner.detach(Box::pin(logger));
}

/// Each stream `reader` leaves unread is closed after `IDLE`, unless read again by then.
pub(super) fn release(state: &mut AppState, spawner: &dyn Spawner, reader: &str) {
    for key in state.cluster.store.logs.release(reader) {
        let sink = spawner.sink();
        spawner.detach(Box::pin(async move {
            tokio::time::sleep(IDLE).await;
            sink.deliver(Box::new(
                move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                    state.cluster.store.logs.drop_unread(&key)
                },
            ));
        }));
    }
}

/// What the caret holds in the stream's text, to the clipboard.
pub(super) fn copy(state: &AppState, services: &Services, spawner: &dyn Spawner, key: &LogKey) {
    let held = state.cluster.store.logs.get(key);
    let said = held.map(|one| one.buffer().selected()).unwrap_or_default();
    crate::workspace::copied(services, spawner, said);
}
