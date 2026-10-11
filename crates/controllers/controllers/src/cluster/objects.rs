//! A context's kinds, and the watchers its readers hold: started with the first, stopped after the last.

use std::time::Duration;

use groove_cluster_service::Event as ClusterEvent;
use groove_types::{Error, WatchKey};

use crate::{AppState, Continuation, Event, Services, Spawner, apply};

/// How long a watcher left unread keeps running.
pub(super) const IDLE: Duration = Duration::from_secs(30);

/// The wait after a first failed discovery, doubled with each failure after it up to `LONGEST`.
const RETRY: Duration = Duration::from_secs(1);
const LONGEST: Duration = Duration::from_secs(60);

pub(super) fn discover(state: &mut AppState, spawner: &dyn Spawner, context: String, again: bool) {
    if !known(state, &context) || !state.cluster.store.begin_discovery(&context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    let cache = state.env.cache_dir.join("kube");
    spawner.spawn(Box::pin(async move {
        let kinds = groove_cluster_service::kinds(paths, context.clone(), cache, again).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| match kinds {
                Ok(kinds) => apply(
                    Event::Cluster(ClusterEvent::Kinds { context, kinds }),
                    state,
                ),
                Err(e) => refused(state, spawner, context, e),
            },
        ) as Continuation
    }));
}

/// A failed discovery: said the first time, tried again after a wait that doubles.
fn refused(state: &mut AppState, spawner: &dyn Spawner, context: String, error: Error) {
    let failures = state.cluster.store.refuse_discovery(&context);
    if failures == 1 {
        state.failed(error);
    }
    let wait = RETRY
        .saturating_mul(1 << failures.saturating_sub(1).min(16))
        .min(LONGEST);
    let sink = spawner.sink();
    spawner.detach(Box::pin(async move {
        tokio::time::sleep(wait).await;
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                state.cluster.store.retry_discovery(&context, failures)
            },
        ));
    }));
}

pub(super) fn namespaces(state: &mut AppState, spawner: &dyn Spawner, context: String) {
    if !known(state, &context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let names = groove_cluster_service::namespaces(paths, context.clone()).await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match names {
                Ok(names) => state.cluster.store.set_namespaces(&context, names),
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}

pub(super) fn watch(state: &mut AppState, spawner: &dyn Spawner, reader: &str, key: WatchKey) {
    if !known(state, &key.context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    let Some(stop) = state.cluster.store.lease(&key, reader) else {
        return;
    };
    let (sink, held) = (spawner.sink(), key.clone());
    let send = move |batch| {
        let key = Box::new(held.clone());
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                apply(Event::Cluster(ClusterEvent::Watched { key, batch }), state)
            },
        ));
    };
    let watcher = groove_cluster_service::watcher(paths, key, stop, send);
    spawner.detach(Box::pin(watcher));
}

/// Each watcher `reader` leaves unread is dropped after `IDLE`, unless read again by then.
pub(super) fn release(state: &mut AppState, spawner: &dyn Spawner, reader: &str) {
    for key in state.cluster.store.release(reader) {
        let sink = spawner.sink();
        spawner.detach(Box::pin(async move {
            tokio::time::sleep(IDLE).await;
            sink.deliver(Box::new(
                move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                    state.cluster.store.drop_unread(&key)
                },
            ));
        }));
    }
}

pub(super) fn known(state: &mut AppState, context: &str) -> bool {
    if state.config.cluster(context).is_some() {
        return true;
    }
    let why = format!(
        "Groove does not know the context `{context}`: add it in Settings › Clusters first"
    );
    state.failed(Error::invalid(why));
    false
}
