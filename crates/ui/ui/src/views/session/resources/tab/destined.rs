//! Where an Application's resources stand: the context its destination names, and the links into it.

use groove_controllers::{AppState, Command, cluster};
use groove_types::{AppPart, AppResource, Destination};

use super::super::around::served;
use super::super::opened::Link;

/// The context the destination stands on: the Application's own in-cluster, else the one added whose server it names.
pub(super) fn context(app: &AppState, own: &str, to: &Destination) -> Option<String> {
    let known = |name: &str| app.config.cluster(name).is_some();
    app.cluster.destination(own, to, known)
}

/// The tab a resource of the Application opens, once its context's kinds are known.
pub(super) fn link(app: &AppState, context: Option<&str>, one: &AppResource) -> Option<Link> {
    let context = context?;
    Some(Link {
        context: context.to_string(),
        kind: served(app, context, &one.group, &one.kind)?,
        namespace: one.namespace.clone(),
        name: one.name.clone(),
    })
}

/// The kubeconfig read once when the destination names a server, then the destination's kinds discovered.
pub(super) fn asks(app: &AppState, own: &str, part: &AppPart) -> Vec<Command> {
    let to = &part.destination;
    let Some(context) = context(app, own, to) else {
        let named = to.server.is_some();
        let unread = app.cluster.found.is_none() && !app.cluster.scanned;
        return match named && unread {
            true => vec![Command::Cluster(cluster::Command::ReadContexts)],
            false => Vec::new(),
        };
    };
    let store = &app.cluster.store;
    if store.kinds(&context).is_some() || store.discovering(&context) {
        return Vec::new();
    }
    let discover = cluster::Command::Discover {
        context,
        again: false,
    };
    vec![Command::Cluster(discover)]
}
