//! A user's shell in a pod: a tab named after it, refused on a read-only context.

use groove_types::{Attached, ClusterChange, KubeKind, SessionId, Target};

use crate::shell::Command;
use crate::tests::fixture::{self, kubeconfig};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

/// An explorer holding `paxone` of the known `kind` context.
fn session(home: &std::path::Path, parts: (&mut AppState, &Services, &SyncSpawner)) -> SessionId {
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
    let open = state.session.get_mut(&id).expect("the explorer");
    open.clusters.push(Attached {
        context: "kind".into(),
        namespace: Some("paxone".into()),
    });
    id
}

fn shell(session: &SessionId) -> Cmd {
    let kind = KubeKind {
        group: String::new(),
        version: "v1".into(),
        kind: "Pod".into(),
        plural: "pods".into(),
        namespaced: true,
        watchable: true,
        short_names: Vec::new(),
    };
    let pod = Target {
        context: "kind".into(),
        kind,
        namespace: Some("paxone".into()),
        name: "db-0".into(),
        subresource: None,
    };
    Cmd::Shell(Command::Pod {
        session: session.clone(),
        pod: Box::new(pod),
        container: Some("postgres".into()),
    })
}

#[test]
fn a_pod_s_shell_opens_in_a_tab_named_after_its_container() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = session(home.path(), (&mut state, &services, &spawner));
    dispatch(shell(&id), &mut state, &services, &spawner);
    let shells = state.shell.shells(&id).expect("a tab");
    let tab = shells.tab().expect("the tab up");
    assert_eq!(tab.name.as_deref(), Some("db-0 · postgres"));
    assert_eq!(tab.panes.len(), 1);
}

#[test]
fn a_read_only_context_refuses_a_pod_s_shell() {
    let (home, spawner, services, mut state) = fixture::fresh();
    let id = session(home.path(), (&mut state, &services, &spawner));
    let set = crate::config::Command::SetCluster {
        context: "kind".into(),
        change: ClusterChange::ReadOnly(true),
    };
    dispatch(Cmd::Config(set), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    dispatch(shell(&id), &mut state, &services, &spawner);
    assert!(state.shell.shells(&id).is_none(), "no tab");
    let told = state
        .errors
        .iter()
        .any(|one| one.what.message.contains("read-only"));
    assert!(told, "{:?}", state.errors);
}
