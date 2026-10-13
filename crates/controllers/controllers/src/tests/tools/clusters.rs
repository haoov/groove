//! The agent's cluster tools: reads checked against the session, attach and detach through the queue.

use serde_json::json;

use super::{asked, waited, waiting};
use crate::tests::fixture::{self, kubeconfig};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

/// An explorer open, and the kubeconfig's `kind` context added to Groove.
fn known(
    home: &std::path::Path,
    (state, services, spawner): (&mut AppState, &Services, &SyncSpawner),
) -> String {
    kubeconfig(home, "https://127.0.0.1:1");
    let scan = Cmd::Cluster(crate::cluster::Command::ScanContexts);
    dispatch(scan, state, services, spawner);
    spawner.drain(state, services);
    let add = crate::config::Command::AddCluster {
        context: "kind".into(),
    };
    dispatch(Cmd::Config(add), state, services, spawner);
    fixture::explorer(state, services, spawner)
        .as_str()
        .to_string()
}

#[test]
fn a_read_of_a_context_the_session_does_not_hold_is_refused() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = known(home.path(), (&mut state, &services, &spawner));
    let read = json!({ "context": "kind", "args": ["get", "pods"] });
    let answer = asked(&mut state, &services, &spawner, &id, "kube", read);
    assert!(
        answer.failed && answer.text.contains("no context kind"),
        "{}",
        answer.text
    );
}

#[test]
fn a_read_takes_its_arguments_as_a_list() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = known(home.path(), (&mut state, &services, &spawner));
    let read = json!({ "context": "kind", "args": "get pods" });
    let answer = asked(&mut state, &services, &spawner, &id, "kube", read);
    assert!(
        answer.failed && answer.text.contains("a list of strings"),
        "{}",
        answer.text
    );
}

#[test]
fn an_attach_waits_on_the_user_then_the_session_reads_the_cluster() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = known(home.path(), (&mut state, &services, &spawner));
    let attach = json!({ "context": "kind", "namespace": "api" });
    let pending = waited(
        &mut state,
        &services,
        &spawner,
        &id,
        "attach_cluster",
        attach.clone(),
    );
    assert!(pending.is_none(), "the attach waits on the user");
    assert_eq!(waiting(&state, &id).len(), 1);

    fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));
    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "attach_cluster",
        attach,
    );
    assert!(!answer.failed, "{}", answer.text);
    let open = state
        .session
        .get(&groove_types::SessionId::new(&id))
        .unwrap();
    assert!(
        open.clusters
            .iter()
            .any(|one| one.context == "kind" && one.namespace.as_deref() == Some("api"))
    );

    let write = json!({ "context": "kind", "args": ["delete", "pod", "api-0", "-n", "api"] });
    let refused = asked(&mut state, &services, &spawner, &id, "kube", write);
    assert!(
        refused.failed && refused.text.contains("the reads are"),
        "{}",
        refused.text
    );
    let other = json!({ "context": "kind", "args": ["get", "pods", "-n", "kube-system"] });
    let refused = asked(&mut state, &services, &spawner, &id, "kube", other);
    assert!(
        refused.failed && refused.text.contains("kube-system"),
        "{}",
        refused.text
    );
}

#[test]
fn a_context_groove_does_not_know_is_never_attached() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = known(home.path(), (&mut state, &services, &spawner));
    fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));
    let attach = json!({ "context": "elsewhere" });
    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "attach_cluster",
        attach,
    );
    assert!(
        answer.failed && answer.text.contains("Settings"),
        "{}",
        answer.text
    );
}

#[test]
fn only_what_the_session_holds_is_let_go() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = known(home.path(), (&mut state, &services, &spawner));
    fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));
    let held = json!({ "context": "kind", "namespace": "api" });
    asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "attach_cluster",
        held.clone(),
    );
    let other = json!({ "context": "kind", "namespace": "web" });
    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "detach_cluster",
        other,
    );
    assert!(
        answer.failed && answer.text.contains("kind/api"),
        "{}",
        answer.text
    );
    let answer = asked(&mut state, &services, &spawner, &id, "detach_cluster", held);
    assert!(!answer.failed, "{}", answer.text);
    let open = state
        .session
        .get(&groove_types::SessionId::new(&id))
        .unwrap();
    assert!(open.clusters.is_empty());
}
