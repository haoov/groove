//! What an object's tab reads while it shows: the object, its events, its owners, and around a pod.

use std::collections::BTreeSet;

use groove_controllers::cluster_service::Named;
use groove_controllers::{AppState, Command, cluster, session};
use groove_types::{FollowKey, LogKey, Timestamp};

use super::super::around::{helm_release, lineage};
use super::super::opened::Opened;
use super::super::{ResourcesUi, View, log_key};

/// The one reader an object's tab holds its watchers under.
pub const READER: &str = "resource";

/// How often a pod's usage is read again while its tab shows.
pub const USAGE_EVERY: i64 = 15;

/// How often the commits past an Application's synced revision are counted again.
pub const AHEAD_EVERY: i64 = 60;

/// The watchers the tab shown wants, and the reads it lacks; `up` while the tab is on screen.
pub fn wants(app: &AppState, up: bool, held: &ResourcesUi, now: Timestamp) -> Vec<Command> {
    let added = |tab: &&Opened| app.config.cluster(&tab.link.context).is_some();
    let tab = held.tab().filter(|_| up).filter(added);
    let mut out = Vec::new();
    let (keys, streams) = match tab {
        Some(tab) => {
            out.extend(asks(app, tab, now));
            (keys(app, tab), (logged(app, tab), treed(app, tab)))
        }
        None => (Vec::new(), (None, None)),
    };
    out.extend(leases(app, keys, streams));
    out
}

/// The Application whose resource health the tab reads while its resources show.
fn treed(app: &AppState, tab: &Opened) -> Option<Named> {
    let shown = tab.view == View::Resources && tab.link.read(app)?.app.is_some();
    shown.then(|| tab.link.pod())
}

/// The log stream the tab reads while its logs show.
fn logged(app: &AppState, tab: &Opened) -> Option<LogKey> {
    (tab.view == View::Logs).then(|| log_key(app, tab))?
}

fn keys(app: &AppState, tab: &Opened) -> Vec<FollowKey> {
    let link = &tab.link;
    let mut out = vec![link.key()];
    let Some(object) = link.read(app) else {
        return out;
    };
    let (store, namespace) = (&app.cluster.store, link.namespace.as_deref());
    out.extend(store.events_key(&link.context, namespace, &object.uid));
    out.extend(
        lineage(app, link, object)
            .into_iter()
            .map(|(up, _)| up.key()),
    );
    if object.pod.is_some() {
        out.extend(store.services_key(&link.context, namespace));
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
    let pod = link.pod();
    let stale = follows
        .usage(&pod)
        .is_none_or(|(_, at)| due_at(*at, USAGE_EVERY, now));
    let described = tab.view == View::Describe;
    if object.pod.is_some() && described && stale && !follows.asking(&pod) {
        out.push(Command::Cluster(cluster::Command::Usage {
            pod: pod.clone(),
        }));
    }
    if let Some(part) = &object.app {
        out.extend(super::destined::asks(app, &link.context, part));
        out.extend(ahead(app, part, now));
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

/// The followers and the streams held while they are the ones wanted; else the new ones.
fn leases(
    app: &AppState,
    wanted: Vec<FollowKey>,
    (log, tree): (Option<LogKey>, Option<Named>),
) -> Vec<Command> {
    let store = &app.cluster.store;
    let held: BTreeSet<&FollowKey> = store.follows.read_by(READER).collect();
    let logs: Vec<&LogKey> = store.logs.read_by(READER).collect();
    let trees: Vec<&Named> = store.trees.read_by(READER).collect();
    let same = logs == log.iter().collect::<Vec<_>>()
        && trees == tree.iter().collect::<Vec<_>>()
        && held == wanted.iter().collect();
    let follow = |key| {
        Command::Cluster(cluster::Command::Follow {
            reader: READER.into(),
            key: Box::new(key),
        })
    };
    let stream = log.map(|key| {
        Command::Cluster(cluster::Command::FollowLogs {
            reader: READER.into(),
            key: Box::new(key),
        })
    });
    let treed = tree.map(|app| {
        Command::Cluster(cluster::Command::FollowArgo {
            reader: READER.into(),
            app,
        })
    });
    let reads = wanted.into_iter().map(follow).chain(stream).chain(treed);
    super::super::leases(READER, same, reads)
}

/// The count of commits past the synced revision, when due.
fn ahead(app: &AppState, part: &groove_types::AppPart, now: Timestamp) -> Option<Command> {
    let ahead = part.tracked()?;
    let session = &app.session;
    let held = session.ahead.get(&ahead);
    let stale = held.is_none_or(|(_, at)| due_at(*at, AHEAD_EVERY, now));
    let asking = session.asking_ahead.contains(&ahead);
    (stale && !asking).then(|| Command::Session(session::Command::AheadOf { ahead }))
}

fn due_at(at: Timestamp, every: i64, now: Timestamp) -> bool {
    now.seconds() - at.seconds() >= every
}

/// When the tab shown next wants a pod's usage read, or an Application's commits ahead counted.
pub fn due(app: &AppState, held: &ResourcesUi) -> Option<Timestamp> {
    let tab = held.tab()?;
    let object = tab.link.read(app)?;
    if let Some(part) = object.app.as_deref() {
        let (_, at) = app.session.ahead.get(&part.tracked()?)?;
        return Some(Timestamp::new(at.seconds() + AHEAD_EVERY));
    }
    if tab.view != View::Describe || object.pod.is_none() {
        return None;
    }
    let (_, at) = app.cluster.store.follows.usage(&tab.link.pod())?;
    Some(Timestamp::new(at.seconds() + USAGE_EVERY))
}
