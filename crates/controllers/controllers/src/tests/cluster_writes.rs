//! Writes to a session's clusters: refused on a read-only or unheld context, said on the timeline once stored.

use groove_types::{Attached, ClusterChange, KubeKind, SessionId, Target, TimelineKind};
use serde_json::json;
use wiremock::matchers::{method, path, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::cluster::{Command, Patched};
use crate::config::Command as Config;
use crate::tests::fixture::{self, kubeconfig, until};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

const API: &str = "/apis/apps/v1/namespaces/paxone/deployments/api";

/// The `kind` context added to Groove, on `server`, and an explorer holding `held`.
fn session(
    home: &std::path::Path,
    server: &str,
    held: Option<Attached>,
    (state, services, spawner): (&mut AppState, &Services, &SyncSpawner),
) -> SessionId {
    kubeconfig(home, server);
    let mut run = |command| {
        dispatch(command, state, services, spawner);
        spawner.drain(state, services);
    };
    run(Cmd::Cluster(Command::ScanContexts));
    run(Cmd::Config(Config::AddCluster {
        context: "kind".into(),
    }));
    let id = fixture::explorer(state, services, spawner);
    let open = state.session.get_mut(&id).expect("the explorer");
    open.clusters.extend(held);
    id
}

fn scaled(session: &SessionId) -> Cmd {
    let target = Target {
        context: "kind".into(),
        kind: KubeKind {
            group: "apps".into(),
            version: "v1".into(),
            kind: "Deployment".into(),
            plural: "deployments".into(),
            namespaced: true,
            watchable: true,
        },
        namespace: Some("paxone".into()),
        name: "api".into(),
        subresource: None,
    };
    let patched = Patched {
        target,
        body: json!({ "spec": { "replicas": 3 } }),
        version: Some("7".into()),
    };
    Cmd::Cluster(Command::Patch {
        session: session.clone(),
        patched: Box::new(patched),
    })
}

fn paxone() -> Option<Attached> {
    Some(Attached {
        context: "kind".into(),
        namespace: Some("paxone".into()),
    })
}

/// A server that stores one patch on `api`, and expects `calls` of them.
fn server(spawner: &SyncSpawner, calls: u64) -> MockServer {
    spawner.block_on(async {
        let server = MockServer::start().await;
        let object = json!({ "metadata": { "name": "api", "resourceVersion": "8" } });
        Mock::given(method("PATCH"))
            .and(path(API))
            .and(query_param_is_missing("dryRun"))
            .respond_with(ResponseTemplate::new(200).set_body_json(object))
            .expect(calls)
            .mount(&server)
            .await;
        server
    })
}

#[test]
fn a_patch_on_a_held_context_is_stored_and_said_on_the_timeline() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let server = server(&spawner, 1);
    let parts = (&mut state, &services, &spawner);
    let id = session(home.path(), &server.uri(), paxone(), parts);
    dispatch(scaled(&id), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.session
            .feed
            .iter()
            .any(|one| one.kind == TimelineKind::ClusterWrite)
    });
    let line = state
        .session
        .feed
        .iter()
        .find(|one| one.kind == TimelineKind::ClusterWrite);
    let subject = &line.expect("the write's line").subject;
    assert!(
        subject.starts_with("kind · paxone/deployments/api · patch"),
        "{subject}"
    );
    assert!(subject.contains("\"replicas\":3"), "{subject}");
}

#[test]
fn a_read_only_context_refuses_the_patch_before_any_request() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let server = server(&spawner, 0);
    let parts = (&mut state, &services, &spawner);
    let id = session(home.path(), &server.uri(), paxone(), parts);
    let set = Config::SetCluster {
        context: "kind".into(),
        change: ClusterChange::ReadOnly(true),
    };
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    dispatch(scaled(&id), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let told = state
        .errors
        .iter()
        .any(|one| one.what.message.contains("read-only"));
    assert!(told, "{:?}", state.errors);
}

#[test]
fn a_context_the_session_does_not_hold_refuses_the_patch() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let server = server(&spawner, 0);
    let parts = (&mut state, &services, &spawner);
    let id = session(home.path(), &server.uri(), None, parts);
    dispatch(scaled(&id), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let told = state
        .errors
        .iter()
        .any(|one| one.what.message.contains("no context kind"));
    assert!(told, "{:?}", state.errors);
}
