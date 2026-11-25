//! The kubeconfig contexts: found by a scan, each checked for a sign-in.

use groove_cluster_service::Event as ClusterEvent;

use super::objects;
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

/// One scan at a time; a second ask while it runs is the same answer.
/// The contexts read; with `check`, every context added to Groove checked again.
pub(super) fn scan(state: &mut AppState, spawner: &dyn Spawner, check: bool) {
    if !state.cluster.begin_scan() {
        if check {
            check_added(state, spawner);
        }
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let found = groove_cluster_service::scan(paths).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| match found {
                Ok(contexts) => {
                    apply(Event::Cluster(ClusterEvent::Found(contexts)), state);
                    if check {
                        check_added(state, spawner);
                    }
                }
                Err(e) => {
                    apply(
                        Event::Cluster(ClusterEvent::Unread(e.message.clone())),
                        state,
                    );
                    state.failed(e);
                    unscanned(spawner);
                }
            },
        ) as Continuation
    }));
}

/// A scan that failed may be asked again once `IDLE` has passed.
fn unscanned(spawner: &dyn Spawner) {
    let sink = spawner.sink();
    spawner.detach(Box::pin(async move {
        tokio::time::sleep(objects::IDLE).await;
        sink.deliver(Box::new(
            |state: &mut AppState, _: &Services, _: &dyn Spawner| state.cluster.scanned = false,
        ));
    }));
}

/// Every context added to Groove, checked again.
fn check_added(state: &mut AppState, spawner: &dyn Spawner) {
    let added: Vec<String> = state
        .config
        .clusters()
        .iter()
        .map(|one| one.context.clone())
        .collect();
    for context in added {
        check(state, spawner, context);
    }
}

/// One check a context at a time.
pub(crate) fn check(state: &mut AppState, spawner: &dyn Spawner, context: String) {
    if !state.cluster.begin_check(&context) {
        return;
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let login = groove_cluster_service::check(paths, context.clone()).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(
                Event::Cluster(ClusterEvent::Checked { context, login }),
                state,
            )
        }) as Continuation
    }));
}
