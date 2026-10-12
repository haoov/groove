//! What an object open in a tab reads around itself, from the cluster store.

use std::sync::Arc;

use groove_controllers::AppState;
use groove_types::Described;

use super::opened::Link;

/// The object a row of the list stands for, by its uid.
pub fn row_link(
    app: &AppState,
    open: &groove_controllers::session_service::Open,
    held: &super::ResourcesUi,
    uid: &str,
) -> Option<Link> {
    for key in super::scope::keys(app, open, held) {
        let Some(watched) = app.cluster.store.watched(&key) else {
            continue;
        };
        if let Some(row) = watched.rows.iter().find(|one| one.uid == uid) {
            return Some(Link {
                context: key.context.clone(),
                kind: key.kind.clone(),
                namespace: row.namespace.clone(),
                name: row.name.clone(),
            });
        }
    }
    None
}

/// The object's owners, nearest first, as far as each has been read.
pub fn lineage(
    app: &AppState,
    link: &Link,
    object: &Described,
) -> Vec<(Link, Option<Arc<Described>>)> {
    let store = &app.cluster.store;
    let ups = store.lineage(&link.context, link.namespace.as_deref(), object);
    let up = |one: groove_controllers::cluster_service::Up| {
        let link = Link {
            context: link.context.clone(),
            kind: one.kind,
            namespace: one.namespace,
            name: one.name,
        };
        (link, one.read)
    };
    ups.into_iter().map(up).collect()
}

/// The Helm release an object or its topmost owner was installed by: its namespace and name.
pub fn helm_release(
    object: &Described,
    owners: &[(Link, Option<Arc<Described>>)],
) -> Option<(String, String)> {
    let top = owners.iter().rev().find_map(|(_, read)| read.clone());
    object.helm_release(top.as_deref())
}
