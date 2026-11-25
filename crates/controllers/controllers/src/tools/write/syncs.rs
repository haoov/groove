//! The agent's sync of an Argo CD Application: its drift read, queued for review, synced once approved.

use std::hash::{DefaultHasher, Hash, Hasher};

use groove_cluster_service::{ArgoApp, ArgoSync, DriftFile, Named, SyncResource};
use serde_json::{Value, json};

use super::Write;
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// How much of the drift the rail's text carries.
const SHOWN: usize = 8 * 1024;

/// The drift the sync would apply, read and queued with what it changes, whatever auto-approve says.
pub(super) fn asked(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let (app, asked) = match scoped(state, &write) {
        Ok(scoped) => scoped,
        Err(why) => return write.reply.failed(why),
    };
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    spawner.spawn(Box::pin(async move {
        let read = groove_cluster_service::argo_drifted_once(paths, app.clone()).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| match read {
                Ok(drifted) => queued((state, spawner), write, &app, asked.of(drifted)),
                Err(why) => write.reply.failed(why),
            },
        ) as Continuation
    }));
}

/// The sync approved, made once the drift is still the one shown.
pub(super) fn run(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let (app, asked) = match scoped(state, &write) {
        Ok(scoped) => scoped,
        Err(why) => return write.reply.failed(why),
    };
    let shown = write.text("fingerprint").map(str::to_string);
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    spawner.spawn(Box::pin(async move {
        let read = groove_cluster_service::argo_drifted_once(paths, app.clone()).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                let now = match read {
                    Ok(drifted) => asked.of(drifted),
                    Err(why) => return write.reply.failed(why),
                };
                if shown.as_deref() != Some(fingerprint(&now.files).as_str()) {
                    let why = "the drift changed since it was shown: read it again and ask again";
                    return write.reply.failed(why);
                }
                let sync = ArgoSync {
                    prune: asked.prune,
                    resources: if asked.named {
                        now.resources
                    } else {
                        Vec::new()
                    },
                };
                let named = (app.hub, app.namespace, app.name);
                let session = write.session;
                let asker = Asker::Agent(write.reply);
                crate::cluster::argo::sync((state, spawner), (session, named), sync, asker);
            },
        ) as Continuation
    }));
}

/// What the agent asks: whether to prune, and the resources it names, if any.
#[derive(Clone)]
struct Asked {
    prune: bool,
    named: bool,
    resources: Vec<(String, Option<String>, String)>,
}

/// The files a sync would change, and the resources they stand for.
struct Kept {
    files: Vec<DriftFile>,
    resources: Vec<SyncResource>,
}

impl Asked {
    /// The drift the sync applies: the resources named, a resource to prune only when pruning.
    fn of(&self, drifted: Vec<(SyncResource, DriftFile)>) -> Kept {
        let kept = drifted.into_iter().filter(|(one, (_, _, desired))| {
            let named = !self.named || self.resources.iter().any(|asked| matches(asked, one));
            named && (self.prune || !desired.is_empty())
        });
        let (resources, files) = kept.unzip();
        Kept { files, resources }
    }
}

fn matches((kind, namespace, name): &(String, Option<String>, String), one: &SyncResource) -> bool {
    kind.eq_ignore_ascii_case(&one.kind) && *namespace == one.namespace && *name == one.name
}

/// The Application on the hub the session holds, the sync allowed there, and what it asks.
fn scoped(state: &AppState, write: &Write) -> Result<(ArgoApp, Asked), String> {
    let named = write
        .text("app")
        .ok_or("argo_sync needs app: the Application as <namespace>/<name>")?;
    let open = state
        .session
        .get(&write.session)
        .ok_or(crate::tools::NO_SESSION)?;
    let hub = state.config.clusters().iter().find(|one| one.argo_hub);
    let hub = hub.map(|one| one.context.as_str());
    let app = crate::tools::argo::scoped(named, hub, &open.clusters)?;
    let held: Named = (app.hub.clone(), app.namespace.clone(), app.name.clone());
    crate::cluster::argo::may_sync(state, &write.session, &held).map_err(|e| e.message)?;
    let listed = write.arguments["resources"].as_array().cloned();
    let resources = listed.unwrap_or_default();
    let resources: Result<Vec<_>, String> = resources.iter().map(resource).collect();
    let asked = Asked {
        prune: write.flag("prune").unwrap_or(false),
        named: !resources.as_ref().map_or(true, Vec::is_empty),
        resources: resources?,
    };
    Ok((app, asked))
}

/// `Deployment/shop/web`, or `Namespace/shop` for one no namespace holds.
fn resource(named: &Value) -> Result<(String, Option<String>, String), String> {
    let text = named.as_str().unwrap_or_default();
    let parts: Vec<&str> = text.split('/').collect();
    match parts.as_slice() {
        [kind, namespace, name] => Ok((
            kind.to_string(),
            Some(namespace.to_string()),
            name.to_string(),
        )),
        [kind, name] => Ok((kind.to_string(), None, name.to_string())),
        _ => Err(format!(
            "name a resource as <Kind>/<namespace>/<name>, not {text:?}"
        )),
    }
}

/// The sync waits with the drift it would apply; nothing to apply is said at once.
fn queued(
    (state, spawner): (&mut AppState, &dyn Spawner),
    mut write: Write,
    app: &ArgoApp,
    kept: Kept,
) {
    let target = format!("{} · {}/{}", app.hub, app.namespace, app.name);
    if kept.files.is_empty() {
        return write
            .reply
            .said(format!("nothing to sync on {target}: it is in sync"));
    }
    write.arguments["target"] = target.into();
    write.arguments["changes"] = json!(kept.files);
    write.arguments["fingerprint"] = fingerprint(&kept.files).into();
    write.arguments["diff"] = shown(&kept.files).into();
    if state.agent.agent(&write.session).is_none() {
        return write
            .reply
            .failed("the session closed before argo_sync ran");
    }
    super::queued(state, write);
    crate::workspace::review::show(state, spawner);
}

/// The drift as one number: a sync runs only on the drift its user saw.
fn fingerprint(files: &[DriftFile]) -> String {
    let mut hasher = DefaultHasher::new();
    files.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// The drift as unified text, cut for the rail.
fn shown(files: &[DriftFile]) -> String {
    let mut out = String::new();
    for (path, old, new) in files {
        let one = groove_workspace_service::unified(path, old, new);
        if out.len() + one.len() > SHOWN {
            out.push_str("… cut");
            break;
        }
        out.push_str(&one);
    }
    out
}
