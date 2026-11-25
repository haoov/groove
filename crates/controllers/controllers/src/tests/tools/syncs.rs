//! An Application's sync: refused before any request where the hub is not one, not held or read-only.

use groove_types::{Attached, ClusterChange};
use serde_json::json;

use super::{asked, waiting};
use crate::tests::fixture::{self, kubeconfig};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

/// The `kind` context added, set as `changes` say, and an explorer holding its `argocd` namespace when `held`.
fn session(
    home: &std::path::Path,
    changes: &[ClusterChange],
    held: bool,
    (state, services, spawner): (&mut AppState, &Services, &SyncSpawner),
) -> String {
    kubeconfig(home, "https://127.0.0.1:9");
    let mut run = |command| {
        dispatch(command, state, services, spawner);
        spawner.drain(state, services);
    };
    run(Cmd::Cluster(crate::cluster::Command::ScanContexts));
    run(Cmd::Config(crate::config::Command::AddCluster {
        context: "kind".into(),
    }));
    for change in changes {
        run(Cmd::Config(crate::config::Command::SetCluster {
            context: "kind".into(),
            change: change.clone(),
        }));
    }
    let id = fixture::explorer(state, services, spawner);
    fixture::auto_approve(state, &id);
    let open = state.session.get_mut(&id).expect("the explorer");
    if held {
        open.clusters.push(Attached {
            context: "kind".into(),
            namespace: Some("argocd".into()),
        });
    }
    id.as_str().to_string()
}

#[test]
fn a_read_only_hub_refuses_the_agent_s_sync_before_it_waits() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let changes = [ClusterChange::ArgoHub(true), ClusterChange::ReadOnly(true)];
    let id = session(
        home.path(),
        &changes,
        true,
        (&mut state, &services, &spawner),
    );
    let app = json!({ "app": "argocd/shop" });
    let answer = asked(&mut state, &services, &spawner, &id, "argo_sync", app);
    assert!(
        answer.failed && answer.text.contains("read-only"),
        "{}",
        answer.text
    );
    assert!(
        waiting(&state, &id).is_empty(),
        "nothing waits, even under auto-approve"
    );
}

#[test]
fn a_hub_the_session_does_not_hold_refuses_the_agent_s_sync() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let changes = [ClusterChange::ArgoHub(true)];
    let id = session(
        home.path(),
        &changes,
        false,
        (&mut state, &services, &spawner),
    );
    let app = json!({ "app": "argocd/shop", "resources": ["Deployment/shop/web"] });
    let answer = asked(&mut state, &services, &spawner, &id, "argo_sync", app);
    assert!(
        answer.failed && answer.text.contains("attach it first"),
        "{}",
        answer.text
    );
}

#[test]
fn a_resource_named_badly_is_refused_with_how_to_name_it() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let changes = [ClusterChange::ArgoHub(true)];
    let id = session(
        home.path(),
        &changes,
        true,
        (&mut state, &services, &spawner),
    );
    let app = json!({ "app": "argocd/shop", "resources": ["web"] });
    let answer = asked(&mut state, &services, &spawner, &id, "argo_sync", app);
    assert!(
        answer.failed && answer.text.contains("<Kind>/<namespace>/<name>"),
        "{}",
        answer.text
    );
}

#[test]
fn the_user_s_sync_off_the_hub_is_refused() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = session(home.path(), &[], true, (&mut state, &services, &spawner));
    let sync = crate::cluster::Command::ArgoSync {
        session: groove_types::SessionId::new(&id),
        app: ("kind".into(), "argocd".into(), "shop".into()),
        prune: false,
    };
    dispatch(Cmd::Cluster(sync), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let told = state
        .errors
        .iter()
        .any(|one| one.what.message.contains("not the Argo CD hub"));
    assert!(told, "{:?}", state.errors);
}
