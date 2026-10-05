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
