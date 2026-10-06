//! Contexts: the kubeconfig files are read off the loop, and each check lands on its context.

use groove_types::Login;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::cluster::Command;
use crate::{Command as Cmd, dispatch};

fn kubeconfig(home: &std::path::Path, server: &str) {
    let yaml = format!(
        "apiVersion: v1
kind: Config
clusters:
- {{name: kind, cluster: {{server: '{server}'}}}}
users:
- {{name: kind, user: {{token: not-a-real-token}}}}
contexts:
- {{name: kind, context: {{cluster: kind, user: kind}}}}
"
    );
    std::fs::create_dir_all(home.join(".kube")).unwrap();
    std::fs::write(home.join(".kube/config"), yaml).unwrap();
}

#[test]
fn a_scan_lists_the_contexts_of_the_kubeconfig_and_one_runs_at_a_time() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    kubeconfig(home.path(), "http://127.0.0.1:9");
    dispatch(
        Cmd::Cluster(Command::ScanContexts),
        &mut state,
        &services,
        &spawner,
    );
    assert!(state.cluster.scanning);
    dispatch(
        Cmd::Cluster(Command::ScanContexts),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    let found = state.cluster.found.as_ref().expect("the scan ended");
    let names: Vec<&str> = found.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["kind"]);
}

#[test]
fn no_kubeconfig_is_no_context_and_no_error() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    dispatch(
        Cmd::Cluster(Command::ScanContexts),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(state.cluster.found, Some(Vec::new()));
    assert!(state.errors.is_empty());
}

#[test]
fn a_check_lands_on_its_context() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let server = spawner.block_on(async {
        let server = MockServer::start().await;
        let version = serde_json::json!({
            "major": "1", "minor": "33", "gitVersion": "v1.33.4", "gitCommit": "",
            "gitTreeState": "", "buildDate": "", "goVersion": "", "compiler": "", "platform": ""
        });
        Mock::given(path("/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(version))
            .mount(&server)
            .await;
        server
    });
    kubeconfig(home.path(), &server.uri());
    let check = Command::CheckContext {
        context: "kind".into(),
    };
    dispatch(Cmd::Cluster(check), &mut state, &services, &spawner);
    assert!(state.cluster.checking("kind"));
    spawner.drain(&mut state, &services);
    let signed_in = Login::SignedIn {
        version: "v1.33.4".into(),
    };
    assert_eq!(state.cluster.login("kind"), Some(&signed_in));
    assert!(!state.cluster.checking("kind"));
}

#[test]
fn a_found_context_is_added_written_changed_and_removed_and_an_unknown_one_refused() {
    use crate::config::Command as Config;
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    kubeconfig(home.path(), "http://127.0.0.1:9");
    let run = |state: &mut crate::AppState, command| {
        dispatch(command, state, &services, &spawner);
        spawner.drain(state, &services);
    };
    run(&mut state, Cmd::Cluster(Command::ScanContexts));
    let add = |context: &str| {
        Cmd::Config(Config::AddCluster {
            context: context.into(),
        })
    };
    run(&mut state, add("prod"));
    assert_eq!(state.errors.len(), 1, "prod is in no kubeconfig");
    run(&mut state, add("kind"));
    assert!(
        state.cluster.login("kind").is_some(),
        "an added context is checked"
    );
    let change = groove_types::ClusterChange::ReadOnly(true);
    let set = Config::SetCluster {
        context: "kind".into(),
        change,
    };
    run(&mut state, Cmd::Config(set));
    let file = std::fs::read_to_string(state.env.config_file()).expect("the file written");
    assert!(file.contains("\"read_only\": true"), "{file}");
    run(
        &mut state,
        Cmd::Config(Config::RemoveCluster {
            context: "kind".into(),
        }),
    );
    assert!(state.config.clusters().is_empty());
}

#[test]
fn a_session_attaches_a_known_context_on_a_namespace_or_whole_and_keeps_it_on_disk() {
    use crate::session::Command as Session;
    use groove_types::Attached;
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    kubeconfig(home.path(), "http://127.0.0.1:9");
    let run = |state: &mut crate::AppState, command| {
        dispatch(command, state, &services, &spawner);
        spawner.drain(state, &services);
    };
    run(
        &mut state,
        Cmd::Session(Session::OpenExplorer { title: None }),
    );
    let session = state.session.selected.clone().expect("the explorer");
    let on = |context: &str, namespace: Option<&str>| Attached {
        context: context.into(),
        namespace: namespace.map(str::to_string),
    };
    let attach = |attached| {
        Cmd::Session(Session::AttachCluster {
            session: session.clone(),
            attached,
        })
    };
    run(&mut state, attach(on("kind", Some("paxone"))));
    assert_eq!(state.errors.len(), 1, "kind is not added to Groove yet");
    run(&mut state, Cmd::Cluster(Command::ScanContexts));
    let add = crate::config::Command::AddCluster {
        context: "kind".into(),
    };
    run(&mut state, Cmd::Config(add));
    run(&mut state, attach(on("kind", Some("paxone"))));
    run(&mut state, attach(on("kind", Some("cnpg"))));
    let held =
        |state: &crate::AppState| state.session.get(&session).expect("open").clusters.clone();
    assert_eq!(
        held(&state),
        [on("kind", Some("paxone")), on("kind", Some("cnpg"))]
    );
    run(&mut state, attach(on("kind", Some(" "))));
    assert_eq!(
        held(&state),
        [on("kind", None)],
        "the whole cluster replaces its namespaces"
    );
    run(&mut state, attach(on("kind", Some("paxone"))));
    assert_eq!(
        state.errors.len(),
        2,
        "a namespace of a cluster held whole is refused"
    );
    let stored = spawner
        .block_on(services.session.contents(&session))
        .expect("contents");
    assert_eq!(stored.clusters, [on("kind", None)]);
    let detach = Session::DetachCluster {
        session: session.clone(),
        attached: on("kind", None),
    };
    run(&mut state, Cmd::Session(detach));
    assert!(held(&state).is_empty());
}
