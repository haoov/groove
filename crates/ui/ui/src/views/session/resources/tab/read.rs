//! What the described view reads before it lays out: the events about the object, the pod's
//! usage, and how wide the key column of its labels and annotations stands.

use std::sync::Arc;

use groove_controllers::AppState;
use groove_types::{Described, Usage};
use groove_ui_kit::base::style::Role;

use super::super::opened::Opened;
use crate::ctx::Ctx;

/// The key column of labels and annotations: as wide as the widest key, half of `room` at most.
pub(super) fn key_width(ctx: &mut Ctx, object: &Described, room: f32) -> f32 {
    let style = ctx.styles.small(Role::Ghost);
    let keys = object
        .labels
        .iter()
        .chain(&object.annotations)
        .map(|(key, _)| key);
    let widest = keys.map(|key| ctx.measure(key, &style)).fold(0.0, f32::max);
    (widest + ctx.tokens.lg).min(room / 2.0)
}

/// The events about the object, warnings first, then the latest.
pub(super) fn events(app: &AppState, tab: &Opened, object: &Described) -> Vec<Arc<Described>> {
    let store = &app.cluster.store;
    let namespace = tab.link.namespace.as_deref();
    let key = store.events_key(&tab.link.context, namespace, &object.uid);
    let held = key.and_then(|key| store.follows.get(&key));
    let mut events: Vec<Arc<Described>> = held.map(|one| one.objects.clone()).unwrap_or_default();
    let rank = |one: &Arc<Described>| {
        let row = one.event.as_ref();
        let warning = row.is_some_and(|row| row.warning);
        (!warning, std::cmp::Reverse(row.and_then(|row| row.last)))
    };
    events.sort_by_key(rank);
    events
}

/// The pod's usage as last read, when the cluster runs a metrics-server.
pub(super) fn usage(app: &AppState, tab: &Opened) -> Option<Usage> {
    app.cluster.store.follows.usage(&tab.link.pod())?.0.clone()
}
