//! A kubeconfig context added to Groove, changed or removed; the file written on the spot.

use groove_types::{ClusterChange, Error};

use crate::{AppState, Spawner};

/// A context the kubeconfig names, added and checked at once; one it does not name is refused.
pub(super) fn add(state: &mut AppState, spawner: &dyn Spawner, context: &str) {
    let found = state.cluster.found.iter().flatten();
    if !found.into_iter().any(|one| one.name == context) {
        let why = format!("no kubeconfig names the context `{context}`");
        return state.failed(Error::invalid(why));
    }
    let config = state.config.add_cluster(context).cloned();
    if super::written(state, config) {
        crate::cluster::check(state, spawner, context.to_string());
    }
}

pub(super) fn remove(state: &mut AppState, context: &str) {
    let config = state.config.remove_cluster(context).cloned();
    super::written(state, config);
}

pub(super) fn change(state: &mut AppState, context: &str, change: ClusterChange) {
    let config = state.config.change_cluster(context, change).cloned();
    super::written(state, config);
}
