//! The clusters the agent attaches to its own session, or lets go of.

use groove_types::Attached;

use super::Write;
use crate::{AppState, Services, Spawner};

/// A context Groove knows, on one namespace or the whole cluster.
pub(super) fn attach(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(attached) = asked(&write) else {
        return write.reply.failed("attach_cluster needs a context");
    };
    if let Err(e) = state.config.known_cluster(&attached.context) {
        return write.reply.failed(e.message);
    }
    if state.session.get(&write.session).is_none() {
        return write.reply.failed(crate::tools::NO_SESSION);
    }
    let said = format!("attached {}", named(&attached));
    crate::session::attach_cluster(state, services, spawner, &write.session, attached);
    write.reply.said(said);
}

/// A context or namespace the session holds, let go.
pub(super) fn detach(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(attached) = asked(&write) else {
        return write.reply.failed("detach_cluster needs a context");
    };
    let Some(open) = state.session.get(&write.session) else {
        return write.reply.failed(crate::tools::NO_SESSION);
    };
    if !open.clusters.contains(&attached) {
        let held: Vec<String> = open.clusters.iter().map(named).collect();
        let why = format!(
            "the session does not hold {}; it holds: {}",
            named(&attached),
            held.join(", ")
        );
        return write.reply.failed(why);
    }
    let said = format!("let go of {}", named(&attached));
    crate::session::detach_cluster(state, services, spawner, &write.session, attached);
    write.reply.said(said);
}

fn asked(write: &Write) -> Option<Attached> {
    Some(Attached {
        context: write.text("context")?.to_string(),
        namespace: write.text("namespace").map(str::to_string),
    })
}

/// `prod/api`, or `prod` for the whole cluster.
fn named(one: &Attached) -> String {
    match &one.namespace {
        Some(namespace) => format!("{}/{namespace}", one.context),
        None => one.context.clone(),
    }
}
