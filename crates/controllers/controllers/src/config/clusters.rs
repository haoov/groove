//! A kubeconfig context added to Groove, changed or removed; the file follows.

use groove_types::{ClusterChange, Error};

use crate::{AppState, Services, Spawner};

/// A context the kubeconfig names, added and checked at once; one it does not name is refused.
pub(super) fn add(state: &mut AppState, services: &Services, spawner: &dyn Spawner, context: &str) {
    if !state.cluster.names(context) {
        let why = format!("no kubeconfig names the context `{context}`");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.add_cluster(context).cloned();
    if super::written(state, services, spawner, config) {
        crate::cluster::check(state, spawner, context.to_string());
    }
}

pub(super) fn remove(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    context: &str,
) {
    let config = state.config.remove_cluster(context).cloned();
    super::written(state, services, spawner, config);
}

pub(super) fn change(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    (context, change): (&str, ClusterChange),
) {
    let config = state.config.change_cluster(context, change).cloned();
    super::written(state, services, spawner, config);
}
