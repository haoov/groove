//! The agent's patches on the session's clusters: tried on the server, queued with their diff, stored once approved.

use groove_cluster_service::{Asked, Preview};
use groove_types::{KubeKind, Target};
use groove_workspace_service::Review;
use serde_json::{Value, json};

use super::Write;
use crate::asker::Asker;
use crate::cluster::Patched;
use crate::cluster::writes::allowed;
use crate::{AppState, Continuation, Services, Spawner};

/// A patch tried on the server, then queued with what it changes, whatever auto-approve says.
pub(super) fn asked(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let (asked, body) = match named(&write) {
        Ok(named) => named,
        Err(why) => return write.reply.failed(why),
    };
    let allowed = match allowed(state, &write.session, &asked.context) {
        Ok(allowed) => allowed,
        Err(e) => return write.reply.failed(e.message),
    };
    let paths = state.env.kubeconfig.clone();
    let cache = state.env.cache_dir.join("kube");
    spawner.spawn(Box::pin(async move {
        let tried = groove_cluster_service::preview(paths, cache, asked, allowed, body).await;
        let reviewed = tried.map(|(target, preview)| {
            let path = path_of(&target);
            let review = groove_workspace_service::review(&path, &preview.before, &preview.after);
            (target, preview, review)
        });
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match reviewed {
                Ok((target, preview, review)) => queued(state, write, &target, (preview, review)),
                Err(e) => write.reply.failed(e.message),
            },
        ) as Continuation
    }));
}

/// The patch the user approved, stored on the version it was shown on.
pub(super) fn patch(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let Some(target) = target(&write) else {
        return write
            .reply
            .failed("kube_patch was approved without the object it was shown on");
    };
    let patched = Patched {
        target,
        body: write.arguments["patch"].clone(),
        version: write.text("resource_version").map(str::to_string),
    };
    let asker = Asker::Agent(write.reply);
    crate::cluster::writes::patch(state, spawner, write.session, patched, asker);
}

fn named(write: &Write) -> Result<(Asked, Value), String> {
    let needs = |name: &str| {
        write
            .text(name)
            .map(str::to_string)
            .ok_or(format!("kube_patch needs the {name}"))
    };
    let asked = Asked {
        context: needs("context")?,
        kind: needs("kind")?,
        namespace: write.text("namespace").map(str::to_string),
        name: needs("name")?,
        subresource: write.text("subresource").map(str::to_string),
    };
    match &write.arguments["patch"] {
        body @ Value::Object(_) => Ok((asked, body.clone())),
        _ => Err("kube_patch needs the patch, a JSON object".to_string()),
    }
}

/// The write waits with its target, its diff and the version it was tried on.
fn queued(
    state: &mut AppState,
    mut write: Write,
    target: &Target,
    (preview, review): (Preview, Review),
) {
    if preview.diff.is_empty() {
        let said = format!("the patch changes nothing on {}", target.label());
        return write.reply.said(said);
    }
    let kind = &target.kind;
    write.arguments["target"] = target.label().into();
    write.arguments["diff"] = preview.diff.into();
    write.arguments["resource_version"] = preview.version.into();
    write.arguments["namespace"] = target.namespace.clone().into();
    write.arguments["resolved"] = json!({ "group": kind.group, "version": kind.version,
        "kind": kind.kind, "plural": kind.plural, "namespaced": kind.namespaced });
    if state.agent.agent(&write.session).is_none() {
        return write
            .reply
            .failed("the session closed before kube_patch ran");
    }
    let id = super::queued(state, write);
    state.workspace.reviews.insert(id, review);
}

/// `kind/paxone/deployments/api.yaml`: the key of the review's colours, and its grammar.
fn path_of(target: &Target) -> String {
    let namespace = target.namespace.as_deref().unwrap_or("-");
    let name = match &target.subresource {
        Some(part) => format!("{}-{part}", target.name),
        None => target.name.clone(),
    };
    let plural = &target.kind.plural;
    format!("{}/{namespace}/{plural}/{name}.yaml", target.context)
}

fn target(write: &Write) -> Option<Target> {
    let resolved = &write.arguments["resolved"];
    let text = |name: &str| resolved[name].as_str().map(str::to_string);
    let kind = KubeKind {
        group: text("group")?,
        version: text("version")?,
        kind: text("kind")?,
        plural: text("plural")?,
        namespaced: resolved["namespaced"].as_bool()?,
        watchable: true,
    };
    Some(Target {
        context: write.text("context")?.to_string(),
        kind,
        namespace: write.text("namespace").map(str::to_string),
        name: write.text("name")?.to_string(),
        subresource: write.text("subresource").map(str::to_string),
    })
}
