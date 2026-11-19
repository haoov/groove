//! The agent's commands in a pod: waiting with the command shown, even under auto-approve; refused read-only.

use groove_types::{Attached, ClusterChange};
use serde_json::{Value, json};

use super::{asked, waited, waiting};
use crate::tests::fixture::{self, kubeconfig};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

/// An explorer approving its own writes, holding `paxone` of the known `kind` context.
fn session(home: &std::path::Path, parts: (&mut AppState, &Services, &SyncSpawner)) -> String {
    let (state, services, spawner) = parts;
    kubeconfig(home, "http://127.0.0.1:9");
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

fn psql() -> Value {
    json!({ "context": "kind", "namespace": "paxone", "pod": "db-0",
        "container": "postgres", "command": ["psql", "-c", "select 'a'"] })
}

#[test]
fn a_command_waits_with_what_it_runs_under_auto_approve() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = session(home.path(), (&mut state, &services, &spawner));
    let none = waited(&mut state, &services, &spawner, &id, "kube_exec", psql());
    assert!(none.is_none(), "nothing runs before the user approves");
    let ask = waiting(&state, &id)[0].clone();
    assert!(ask.reviewed, "approved only from the band");
    assert_eq!(ask.subject, "kind · paxone/pods/db-0");
    let shown = ask.text.contains(r"psql -c 'select '\''a'\'''");
    assert!(shown, "{}", ask.text);
    fixture::until(&spawner, &services, &mut state, |s| {
        s.workspace.reviewed() == Some(ask.id.as_str())
    });
    let file = &state.workspace.files[0];
    assert_eq!(file.path, "kind/paxone/db-0/postgres.sh");
}

#[test]
fn a_read_only_context_refuses_a_command_before_it_waits() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = session(home.path(), (&mut state, &services, &spawner));
    let set = crate::config::Command::SetCluster {
        context: "kind".into(),
        change: ClusterChange::ReadOnly(true),
    };
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let answer = asked(&mut state, &services, &spawner, &id, "kube_exec", psql());
    let refused = answer.failed && answer.text.contains("read-only");
    assert!(refused, "{}", answer.text);
    assert!(waiting(&state, &id).is_empty());
}
