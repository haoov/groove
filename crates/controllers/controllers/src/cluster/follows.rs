//! What the resource tabs read: followed objects, usage, Helm revisions, Argo CD health and commits.

use groove_cluster_service::{ArgoApp, Event as ClusterEvent, Named, Revised};
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
                Err(e) => {
                    if follows.fail_usage(pod, Timestamp::now()) {
                        state.failed(e);
                    }
                }
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
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| match read {
                Ok(revision) => state.cluster.store.follows.set_helm(release, revision),
                Err(_) => asked_again(spawner, release),
            },
        ) as Continuation
    }));
}

/// Whether `context` is an Argo CD hub the user added.
fn hub(state: &AppState, context: &str) -> bool {
    let clusters = state.config.clusters();
    clusters
        .iter()
        .any(|one| one.argo_hub && one.context == context)
}

/// One read of a revision; its answer, or why none, stays: a commit does not change.
pub(super) fn revision(state: &mut AppState, spawner: &dyn Spawner, revised: Revised) {
    if !hub(state, &revised.0.0) || !state.cluster.store.revisions.begin(&revised) {
        return;
    }
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    let ((hub, namespace, name), source, sha) = revised.clone();
    let app = ArgoApp {
        hub,
        namespace,
        name,
    };
    spawner.spawn(Box::pin(async move {
        let read = groove_cluster_service::argo_revision_once(paths, app, (source, sha)).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.cluster.store.revisions.set(revised, read)
        }) as Continuation
    }));
}

/// One read of an Application's sync windows at a time; the answer lands with when it was read.
pub(super) fn windows(state: &mut AppState, spawner: &dyn Spawner, named: Named) {
    if !hub(state, &named.0) || !state.cluster.store.windows.begin(&named) {
        return;
    }
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    let (hub, namespace, name) = named.clone();
    let app = ArgoApp {
        hub,
        namespace,
        name,
    };
    spawner.spawn(Box::pin(async move {
        let read = groove_cluster_service::argo_windows_once(paths, app).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state
                .cluster
                .store
                .windows
                .set(named, read, Timestamp::now())
        }) as Continuation
    }));
}

/// The tree stream of an Application on the Argo CD hub, opened with its first reader.
pub(super) fn argo(state: &mut AppState, spawner: &dyn Spawner, reader: &str, app: Named) {
    if !hub(state, &app.0) {
        return;
    }
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    let Some(stop) = state.cluster.store.trees.lease(&app, reader) else {
        return;
    };
    let (sink, held) = (spawner.sink(), app.clone());
    let send = move |read| {
        let app = Box::new(held.clone());
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                apply(Event::Cluster(ClusterEvent::Treed { app, read }), state)
            },
        ));
    };
    let (hub, namespace, name) = app;
    let app = ArgoApp {
        hub,
        namespace,
        name,
    };
    spawner.detach(Box::pin(groove_cluster_service::argo_tree(
        paths, app, stop, send,
    )));
}

/// A failed read stays begun for `IDLE`, then may be asked again.
fn asked_again(spawner: &dyn Spawner, release: Named) {
    let sink = spawner.sink();
    spawner.detach(Box::pin(async move {
        tokio::time::sleep(IDLE).await;
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                state.cluster.store.follows.end(&release)
            },
        ));
    }));
}
