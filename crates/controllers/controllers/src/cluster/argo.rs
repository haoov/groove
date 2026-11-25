//! An Application synced or refreshed through Argo CD on the hub, by the user or an approved agent.

use groove_cluster_service::{ArgoApp, ArgoSync, Named};
use groove_types::{Error, KubeKind, SessionId, Target, TimelineKind};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// An Argo CD command run; any other handed back.
pub(super) fn dispatched(
    command: super::Command,
    state: &mut AppState,
    spawner: &dyn Spawner,
) -> Option<super::Command> {
    use super::Command;
    match command {
        Command::FollowArgo { reader, app } => super::follows::argo(state, spawner, &reader, app),
        Command::ArgoRevision { revised } => super::follows::revision(state, spawner, *revised),
        Command::ArgoWindows { app } => super::follows::windows(state, spawner, app),
        Command::ArgoSync {
            session,
            app,
            prune,
        } => asked((state, spawner), (session, app), prune),
        Command::ArgoRefresh { app, hard } => refresh(state, spawner, app, hard),
        other => return Some(other),
    }
    None
}

/// The user's sync of the whole Application.
fn asked(
    (state, spawner): (&mut AppState, &dyn Spawner),
    (session, app): (SessionId, Named),
    prune: bool,
) {
    let whole = ArgoSync {
        prune,
        resources: Vec::new(),
    };
    sync((state, spawner), (session, app), whole, Asker::Ui);
}

/// `app` synced once the session may: the hub writable and held, its destination not read-only.
pub(crate) fn sync(
    (state, spawner): (&mut AppState, &dyn Spawner),
    (session, app): (SessionId, Named),
    sync: ArgoSync,
    asker: Asker,
) {
    if let Err(e) = may_sync(state, &session, &app) {
        return asker.failed(state, e);
    }
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    let (hub, namespace, name) = app;
    let subject = match sync.prune {
        true => format!("{hub} · sync {namespace}/{name} with prune"),
        false => format!("{hub} · sync {namespace}/{name}"),
    };
    let argo = ArgoApp {
        hub,
        namespace,
        name,
    };
    spawner.spawn(Box::pin(async move {
        let done = groove_cluster_service::argo_sync_once(paths, argo, sync).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match done {
                Ok(()) => {
                    let kind = TimelineKind::ClusterWrite;
                    crate::timeline::said(services, spawner, &session, kind, subject.clone());
                    asker.done(|| format!("sync started: {subject}"));
                }
                Err(why) => asker.failed(state, Error::invalid(why)),
            },
        ) as Continuation
    }));
}

/// `app` compared with its sources again, `hard` past Argo CD's cache; it changes nothing live.
pub(crate) fn refresh(state: &mut AppState, spawner: &dyn Spawner, app: Named, hard: bool) {
    if !super::follows::hub(state, &app.0) {
        let why = format!("{} is not the Argo CD hub", app.0);
        return state.failed(Error::invalid(why));
    }
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    let (hub, namespace, name) = app;
    let argo = ArgoApp {
        hub,
        namespace,
        name,
    };
    spawner.spawn(Box::pin(async move {
        let done = groove_cluster_service::argo_refresh_once(paths, argo, hard).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(why) = done {
                state.failed(Error::invalid(why));
            }
        }) as Continuation
    }));
}

/// The hub writable and held where the Application stands, and its destination, when known, not read-only.
pub(crate) fn may_sync(state: &AppState, session: &SessionId, app: &Named) -> Result<(), Error> {
    let (hub, namespace, name) = app;
    if !super::follows::hub(state, hub) {
        return Err(Error::invalid(format!("{hub} is not the Argo CD hub")));
    }
    let allowed = super::writes::allowed(state, session, hub)?;
    let target = Target {
        context: hub.clone(),
        kind: application(state, hub),
        namespace: Some(namespace.clone()),
        name: name.clone(),
        subresource: None,
    };
    groove_cluster_service::writable(&target, allowed.read_only, &allowed.held)?;
    let to = destination(state, &target);
    let shut = to.as_deref().and_then(|one| state.config.cluster(one));
    match shut.filter(|one| one.read_only) {
        Some(one) => Err(Error::invalid(format!(
            "{} is read-only: turn it off in Settings › Clusters to sync onto it",
            one.context
        ))),
        None => Ok(()),
    }
}

/// The context the Application syncs onto, once its object is read.
fn destination(state: &AppState, target: &Target) -> Option<String> {
    let key = groove_types::FollowKey::named(
        &target.context,
        &target.kind,
        target.namespace.as_deref(),
        &target.name,
    );
    let read = state.cluster.store.object(&key)?;
    let to = &read.app.as_deref()?.destination;
    let known = |name: &str| state.config.cluster(name).is_some();
    state.cluster.destination(&target.context, to, known)
}

/// Argo CD's Application kind as the hub serves it, else as Argo CD defines it.
fn application(state: &AppState, hub: &str) -> KubeKind {
    let served = state
        .cluster
        .store
        .served(hub, "argoproj.io", "Application");
    served.unwrap_or_else(|| KubeKind {
        group: "argoproj.io".into(),
        version: "v1alpha1".into(),
        kind: "Application".into(),
        plural: "applications".into(),
        namespaced: true,
        watchable: true,
        short_names: Vec::new(),
    })
}
