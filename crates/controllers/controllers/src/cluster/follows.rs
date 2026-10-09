//! What the resource tabs read: whole objects followed while read, a pod's usage, a Helm revision.

use groove_cluster_service::{Event as ClusterEvent, Named};
use groove_types::{FollowKey, Timestamp};

use super::objects::{IDLE, known};
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

pub(super) fn follow(state: &mut AppState, spawner: &dyn Spawner, reader: &str, key: FollowKey) {
    if !known(state, &key.context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    let Some(stop) = state.cluster.store.follows.lease(&key, reader) else {
        return;
    };
    let (sink, held) = (spawner.sink(), key.clone());
    let send = move |batch| {
        let key = Box::new(held.clone());
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                apply(Event::Cluster(ClusterEvent::Followed { key, batch }), state)
            },
        ));
    };
    let follower = groove_cluster_service::follower(paths, key, stop, send);
    spawner.detach(Box::pin(follower));
}

/// Each object `reader` leaves unread is dropped after `IDLE`, unless read again by then.
pub(super) fn release(state: &mut AppState, spawner: &dyn Spawner, reader: &str) {
    for key in state.cluster.store.follows.release(reader) {
        let sink = spawner.sink();
        spawner.detach(Box::pin(async move {
            tokio::time::sleep(IDLE).await;
            sink.deliver(Box::new(
                move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                    state.cluster.store.follows.drop_unread(&key)
                },
            ));
        }));
    }
}

/// One read of a pod's usage at a time; the answer lands with when it was read.
pub(super) fn usage(state: &mut AppState, spawner: &dyn Spawner, pod: Named) {
    if !known(state, &pod.0) || !state.cluster.store.follows.begin(&pod) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let read = groove_cluster_service::usage(paths, pod.clone()).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            let follows = &mut state.cluster.store.follows;
            match read {
                Ok(usage) => follows.set_usage(pod, usage, Timestamp::now()),
                Err(_) => follows.set_usage(pod, None, Timestamp::now()),
            }
        }) as Continuation
    }));
}

/// One read of a release's revision; a failure leaves it unknown, to be asked again.
pub(super) fn helm(state: &mut AppState, spawner: &dyn Spawner, release: Named) {
    if !known(state, &release.0) || !state.cluster.store.follows.begin(&release) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let read = groove_cluster_service::helm_revision(paths, release.clone()).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            let follows = &mut state.cluster.store.follows;
            match read {
                Ok(revision) => follows.set_helm(release, revision),
                Err(_) => follows.set_helm(release, None),
            }
        }) as Continuation
    }));
}
