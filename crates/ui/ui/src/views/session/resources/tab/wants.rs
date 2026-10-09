//! What an object's tab reads while it shows: the object, its events, its owners, and around a pod.

use std::collections::BTreeSet;

use groove_controllers::{AppState, Command, cluster};
use groove_types::{FollowKey, Timestamp};

use super::super::ResourcesUi;
use super::super::opened::{Opened, events_key, helm_release, lineage, services_key};

/// The one reader an object's tab holds its watchers under.
pub const READER: &str = "resource";

/// How often a pod's usage is read again while its tab shows.
pub const USAGE_EVERY: i64 = 15;

/// The watchers the tab shown wants, and the reads it lacks; `up` while the tab is on screen.
pub fn wants(app: &AppState, up: bool, held: &ResourcesUi, now: Timestamp) -> Vec<Command> {
    let tab = held.tab().filter(|_| up);
    let mut out = Vec::new();
    let keys = match tab {
        Some(tab) => {
            out.extend(asks(app, tab, now));
            keys(app, tab)
        }
        None => Vec::new(),
    };
    out.extend(leases(app, keys));
    out
}

fn keys(app: &AppState, tab: &Opened) -> Vec<FollowKey> {
    let link = &tab.link;
    let mut out = vec![link.key()];
    let Some(object) = link.read(app) else {
        return out;
    };
    out.extend(events_key(app, link, &object.uid));
    out.extend(
        lineage(app, link, object)
            .into_iter()
            .map(|(up, _)| up.key()),
    );
    if object.pod.is_some() {
        out.extend(services_key(app, link));
    }
    out
}

/// A pod's usage when it is due, and the Helm revision once.
fn asks(app: &AppState, tab: &Opened, now: Timestamp) -> Vec<Command> {
    let link = &tab.link;
    let Some(object) = link.read(app) else {
        return Vec::new();
    };
    let follows = &app.cluster.store.follows;
    let mut out = Vec::new();
    let namespace = link.namespace.clone().unwrap_or_default();
    let pod = (link.context.clone(), namespace, link.name.clone());
    let stale = match follows.usage(&pod) {
        Some((_, at)) => now.seconds() - at.seconds() >= USAGE_EVERY,
        None => true,
    };
    if object.pod.is_some() && !tab.yaml && stale && !follows.asking(&pod) {
        out.push(Command::Cluster(cluster::Command::Usage { pod }));
    }
    let owners = lineage(app, link, object);
    if let Some((namespace, name)) = helm_release(object, &owners) {
        let release = (link.context.clone(), namespace, name);
        if follows.helm(&release).is_none() && !follows.asking(&release) {
            out.push(Command::Cluster(cluster::Command::HelmRevision { release }));
        }
    }
    out
}

/// Nothing while the keys held are the keys wanted; else let go of them all and read the new.
fn leases(app: &AppState, wanted: Vec<FollowKey>) -> Vec<Command> {
    let held: BTreeSet<&FollowKey> = app.cluster.store.follows.read_by(READER).collect();
    if held == wanted.iter().collect() {
        return Vec::new();
    }
    let release = cluster::Command::Release {
        reader: READER.into(),
    };
    let follow = |key| {
        Command::Cluster(cluster::Command::Follow {
            reader: READER.into(),
            key: Box::new(key),
        })
    };
    std::iter::once(Command::Cluster(release))
        .chain(wanted.into_iter().map(follow))
        .collect()
}

/// When the tab shown next wants a pod's usage read.
pub fn due(app: &AppState, held: &ResourcesUi) -> Option<Timestamp> {
    let tab = held.tab().filter(|tab| !tab.yaml)?;
    tab.link.read(app)?.pod.as_ref()?;
    let link = &tab.link;
    let pod = (
        link.context.clone(),
        link.namespace.clone().unwrap_or_default(),
        link.name.clone(),
    );
    let (_, at) = app.cluster.store.follows.usage(&pod)?;
    Some(Timestamp::new(at.seconds() + USAGE_EVERY))
}
