//! The agent's read of an Application's resource health, from Argo CD on the hub the session holds.

use groove_agent_service::Call;
use groove_cluster_service::ArgoApp;
use groove_types::{Attached, ResourceHealth, TimelineKind};

use crate::{AppState, Continuation, Services, Spawner};

/// How much of an answer the agent reads.
const LIMIT: usize = 32 * 1024;

/// Each resource's health in one Application, answered from a job.
pub(super) fn health(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let session = super::session(&call);
    let Some(open) = state.session.get(&session) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let Some(named) = call.text("app").map(str::to_string) else {
        return call
            .reply
            .failed("argo_health needs app: the Application as <namespace>/<name>");
    };
    let hub = state.config.clusters().iter().find(|one| one.argo_hub);
    let app = match scoped(&named, hub.map(|one| one.context.as_str()), &open.clusters) {
        Ok(app) => app,
        Err(why) => return call.reply.failed(why),
    };
    let subject = format!("{} · argo health {named}", app.hub);
    crate::timeline::said(
        services,
        spawner,
        &session,
        TimelineKind::ClusterRead,
        subject,
    );
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    spawner.spawn(Box::pin(async move {
        match groove_cluster_service::argo_tree_once(paths, app).await {
            Ok(tree) => call.reply.said(said(&tree)),
            Err(why) => call.reply.failed(why),
        }
        Box::new(|_: &mut AppState, _: &Services, _: &dyn Spawner| {}) as Continuation
    }));
}

/// What differs between one Application's desired and live state, answered from a job.
pub(super) fn diff(state: &AppState, services: &Services, spawner: &dyn Spawner, call: Call) {
    let session = super::session(&call);
    let Some(open) = state.session.get(&session) else {
        return call.reply.failed(super::NO_SESSION);
    };
    let Some(named) = call.text("app").map(str::to_string) else {
        return call
            .reply
            .failed("argo_diff needs app: the Application as <namespace>/<name>");
    };
    let hub = state.config.clusters().iter().find(|one| one.argo_hub);
    let app = match scoped(&named, hub.map(|one| one.context.as_str()), &open.clusters) {
        Ok(app) => app,
        Err(why) => return call.reply.failed(why),
    };
    let subject = format!("{} · argo diff {named}", app.hub);
    crate::timeline::said(
        services,
        spawner,
        &session,
        TimelineKind::ClusterRead,
        subject,
    );
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    spawner.spawn(Box::pin(async move {
        match groove_cluster_service::argo_drift_once(paths, app).await {
            Ok(files) => call.reply.said(drifted(&files)),
            Err(why) => call.reply.failed(why),
        }
        Box::new(|_: &mut AppState, _: &Services, _: &dyn Spawner| {}) as Continuation
    }));
}

/// Each differing resource's change as text, cut at the agent's limit.
fn drifted(files: &[groove_cluster_service::DriftFile]) -> String {
    let mut out = String::new();
    for (path, old, new) in files {
        let one = groove_workspace_service::unified(path, old, new);
        if out.len() + one.len() > LIMIT {
            out.push_str("… cut at 32 KiB");
            break;
        }
        out.push_str(&one);
    }
    match out.is_empty() {
        true => "in sync: nothing differs".to_string(),
        false => out,
    }
}

/// `namespace/name` on the hub, when the session holds the hub and that namespace.
fn scoped(named: &str, hub: Option<&str>, attached: &[Attached]) -> Result<ArgoApp, String> {
    let Some((namespace, name)) = named.split_once('/') else {
        return Err("name the Application as <namespace>/<name>".into());
    };
    let Some(hub) = hub else {
        return Err(
            "no context is the Argo CD hub: the user marks one in Settings › Clusters".into(),
        );
    };
    let held: Vec<&Attached> = attached.iter().filter(|one| one.context == hub).collect();
    if held.is_empty() {
        return Err(format!(
            "the session does not hold the Argo CD hub {hub}: attach it first"
        ));
    }
    let whole = held.iter().any(|one| one.namespace.is_none());
    if !whole
        && !held
            .iter()
            .any(|one| one.namespace.as_deref() == Some(namespace))
    {
        return Err(format!(
            "the session does not hold the namespace {namespace} on {hub}"
        ));
    }
    Ok(ArgoApp {
        hub: hub.to_string(),
        namespace: namespace.to_string(),
        name: name.to_string(),
    })
}

/// A line a resource that has a health: `Deployment shop/web Degraded: progress deadline exceeded`.
fn said(tree: &[ResourceHealth]) -> String {
    let mut out = String::new();
    for one in tree.iter().filter(|one| !one.status.is_empty()) {
        let namespace = one.namespace.as_deref().unwrap_or_default();
        let mut line = format!("{} {namespace}/{} {}", one.kind, one.name, one.status);
        if !one.reason.is_empty() {
            line.push_str(&format!(": {}", one.reason));
        }
        if out.len() + line.len() > LIMIT {
            out.push_str("… cut at 32 KiB");
            break;
        }
        out.push_str(&line);
        out.push('\n');
    }
    match out.is_empty() {
        true => "no resource of this Application has a health".to_string(),
        false => out,
    }
}
