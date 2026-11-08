//! The agent's patches: queued with their diff even under auto-approve, stored on the version shown.

use groove_agent_service::{Call, Reply};
use groove_types::{Attached, ClusterChange, TimelineKind};
use serde_json::{Value, json};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::{asked, settled, waiting};
use crate::tests::fixture::{self, kubeconfig};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

const API: &str = "/apis/apps/v1/namespaces/paxone/deployments/api";

fn deployment(replicas: u32, version: &str) -> Value {
    json!({ "metadata": { "name": "api", "namespace": "paxone", "resourceVersion": version },
        "spec": { "replicas": replicas } })
}

/// A cluster serving deployments, `api` at version 7 with one replica; a stored patch answers `stored`.
fn server(spawner: &SyncSpawner, stored: u16) -> MockServer {
    spawner.block_on(async {
        let server = MockServer::start().await;
        let plain = |body: Value| ResponseTemplate::new(200).set_body_json(body);
        let mount = |route: &str, body: Value| {
            Mock::given(path(route.to_string())).respond_with(plain(body))
        };
        mount(
            "/api",
            json!({ "kind": "APIVersions", "versions": ["v1"],
            "serverAddressByClientCIDRs": [] }),
        )
        .mount(&server)
        .await;
        mount(
            "/api/v1",
            json!({ "kind": "APIResourceList", "groupVersion": "v1",
            "resources": [] }),
        )
        .mount(&server)
        .await;
        mount(
            "/apis",
            json!({ "kind": "APIGroupList", "groups": [{ "name": "apps",
            "versions": [{ "groupVersion": "apps/v1", "version": "v1" }],
            "preferredVersion": { "groupVersion": "apps/v1", "version": "v1" } }] }),
        )
        .mount(&server)
        .await;
        mount(
            "/apis/apps/v1",
            json!({ "kind": "APIResourceList", "groupVersion": "apps/v1",
            "resources": [{ "name": "deployments", "singularName": "deployment",
                "namespaced": true, "kind": "Deployment",
                "verbs": ["get", "list", "watch", "patch"] }] }),
        )
        .mount(&server)
        .await;
        Mock::given(method("GET"))
            .and(path(API))
            .respond_with(plain(deployment(1, "7")))
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path(API))
            .and(query_param("dryRun", "All"))
            .respond_with(plain(deployment(3, "8")))
            .mount(&server)
            .await;
        let status = json!({ "kind": "Status", "apiVersion": "v1", "status": "Failure",
            "message": "the object has been modified", "reason": "Conflict", "code": 409 });
        let answer = match stored {
            200 => plain(deployment(3, "8")),
            code => ResponseTemplate::new(code).set_body_json(status),
        };
        Mock::given(method("PATCH"))
            .and(path(API))
            .and(query_param_is_missing("dryRun"))
            .respond_with(answer)
            .mount(&server)
            .await;
        server
    })
}

/// An explorer on `paxone` of the known `kind` context, approving its own writes.
fn session(
    home: &std::path::Path,
    server: &MockServer,
    (state, services, spawner): (&mut AppState, &Services, &SyncSpawner),
) -> String {
    kubeconfig(home, &server.uri());
    let mut run = |command| {
        dispatch(command, state, services, spawner);
        spawner.drain(state, services);
    };
    run(Cmd::Cluster(crate::cluster::Command::ScanContexts));
    run(Cmd::Config(crate::config::Command::AddCluster {
        context: "kind".into(),
    }));
    let id = fixture::explorer(state, services, spawner);
    fixture::auto_approve(state, &id);
    let open = state.session.get_mut(&id).expect("the explorer");
    open.clusters.push(Attached {
        context: "kind".into(),
        namespace: Some("paxone".into()),
    });
    id.as_str().to_string()
}

fn scaled() -> Value {
    json!({ "context": "kind", "kind": "deployments", "namespace": "paxone", "name": "api",
        "patch": { "spec": { "replicas": 3 } } })
}

/// The patch asked, left waiting on the user; its answer comes once they decide.
fn queued(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
    id: &str,
) -> tokio::sync::oneshot::Receiver<groove_agent_service::Answer> {
    let (reply, answered) = Reply::new();
    let call = Call {
        session: id.to_string(),
        tool: "kube_patch".into(),
        arguments: scaled(),
        reply,
    };
    crate::tools::answer(state, services, spawner, call);
    fixture::until(spawner, services, state, |s| !waiting(s, id).is_empty());
    answered
}

#[test]
fn a_patch_waits_with_its_diff_under_auto_approve_and_lands_once_approved() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let server = server(&spawner, 200);
    let id = session(home.path(), &server, (&mut state, &services, &spawner));
    let mut answered = queued(&mut state, &services, &spawner, &id);
    let ask = waiting(&state, &id)[0].clone();
    assert_eq!(ask.subject, "kind · paxone/deployments/api");
    assert!(
        ask.text.contains("-  replicas: 1\n+  replicas: 3"),
        "{}",
        ask.text
    );

    assert!(ask.reviewed, "approved only from the sheet");
    let review = state
        .workspace
        .reviews
        .get(&ask.id)
        .expect("the change held");
    assert!(review.aligned.path.ends_with("deployments/api.yaml"));

    crate::tools::allow(&mut state, &services, &spawner, &ask.id);
    assert!(state.workspace.reviews.is_empty(), "let go once decided");
    let answer = settled(&spawner, &services, &mut state, &mut answered);
    assert!(!answer.failed, "{}", answer.text);
    fixture::until(&spawner, &services, &mut state, |s| {
        s.session
            .feed
            .iter()
            .any(|one| one.kind == TimelineKind::ClusterWrite)
    });
    let tried = spawner
        .block_on(server.received_requests())
        .unwrap_or_default();
    let stored = tried.iter().find(|one| {
        one.method.as_str() == "PATCH" && one.url.query().is_some_and(|q| !q.contains("dryRun"))
    });
    let body: Value = serde_json::from_slice(&stored.expect("the stored patch").body).unwrap();
    assert_eq!(
        body["metadata"]["resourceVersion"], "7",
        "on the version shown"
    );
}

#[test]
fn an_object_that_moved_before_the_approval_is_not_overwritten() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let server = server(&spawner, 409);
    let id = session(home.path(), &server, (&mut state, &services, &spawner));
    let mut answered = queued(&mut state, &services, &spawner, &id);
    let ask = waiting(&state, &id)[0].id.clone();
    crate::tools::allow(&mut state, &services, &spawner, &ask);
    let answer = settled(&spawner, &services, &mut state, &mut answered);
    assert!(
        answer.failed && answer.text.contains("changed since it was read"),
        "{}",
        answer.text
    );
}

#[test]
fn a_read_only_context_refuses_the_patch_before_it_is_tried() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let server = server(&spawner, 200);
    let id = session(home.path(), &server, (&mut state, &services, &spawner));
    let set = crate::config::Command::SetCluster {
        context: "kind".into(),
        change: ClusterChange::ReadOnly(true),
    };
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let answer = asked(&mut state, &services, &spawner, &id, "kube_patch", scaled());
    assert!(
        answer.failed && answer.text.contains("read-only"),
        "{}",
        answer.text
    );
    let tried = spawner
        .block_on(server.received_requests())
        .unwrap_or_default();
    assert!(
        tried.iter().all(|one| one.method.as_str() != "PATCH"),
        "nothing was tried"
    );
    assert!(waiting(&state, &id).is_empty());
}
