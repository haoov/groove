//! Where an Application's resources stand: the context its destination names, and the links into it.

use groove_controllers::{AppState, Command, cluster};
use groove_types::{AppPart, AppResource, Destination, KubeContext};

use super::super::opened::{Link, served};

const IN_CLUSTER: &str = "https://kubernetes.default.svc";

/// The context the destination stands on: the Application's own in-cluster, else the one added whose server it names.
pub(super) fn context(app: &AppState, own: &str, to: &Destination) -> Option<String> {
    let server = match (to.server.as_deref(), to.name.as_deref()) {
        (None, Some("in-cluster")) => return Some(own.to_string()),
        (Some(server), _) => normal(server),
        _ => return None,
    };
    if server == IN_CLUSTER {
        return Some(own.to_string());
    }
    let named = |one: &&KubeContext| {
        let at = one.server.as_deref().map(normal);
        at.as_deref() == Some(server.as_str()) && app.config.cluster(&one.name).is_some()
    };
    let found = app.cluster.found.as_deref()?;
    found.iter().find(named).map(|one| one.name.clone())
}

/// A server address as one form: scheme and host in lower case, no default port, no trailing slash.
fn normal(server: &str) -> String {
    let server = server.trim_end_matches('/');
    let (scheme, rest) = server.split_once("://").unwrap_or(("https", server));
    let (host, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let (scheme, host) = (scheme.to_ascii_lowercase(), host.to_ascii_lowercase());
    let port = if scheme == "http" { ":80" } else { ":443" };
    let host = host.strip_suffix(port).unwrap_or(&host);
    format!("{scheme}://{host}{path}")
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
