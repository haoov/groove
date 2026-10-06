use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use groove_types::{KubeAuth, Login};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{check, found, paths};

fn kubeconfig(dir: &Path, server: &str) -> PathBuf {
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
    let file = dir.join("config");
    std::fs::write(&file, yaml).expect("a kubeconfig written");
    file
}

#[test]
fn kubeconfig_set_names_every_file_it_lists_and_unset_names_the_home_one() {
    let home = Path::new("/home/me");
    let listed = paths(Some(OsStr::new("/a/one:/b/two")), home);
    assert_eq!(listed, [PathBuf::from("/a/one"), PathBuf::from("/b/two")]);
    assert_eq!(paths(None, home), [home.join(".kube/config")]);
    assert_eq!(
        paths(Some(OsStr::new("")), home),
        [home.join(".kube/config")]
    );
}

#[tokio::test]
async fn a_found_context_keeps_how_it_signs_in() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = kubeconfig(dir.path(), "http://127.0.0.1:9");
    let contexts = found(&[file]).await.expect("contexts");
    assert_eq!(contexts[0].name, "kind");
    assert_eq!(contexts[0].auth, KubeAuth::Token);
}

#[tokio::test]
async fn a_context_that_answers_is_signed_in_with_its_version() {
    let server = MockServer::start().await;
    let version = serde_json::json!({
        "major": "1", "minor": "33", "gitVersion": "v1.33.4", "gitCommit": "",
        "gitTreeState": "", "buildDate": "", "goVersion": "", "compiler": "", "platform": ""
    });
    Mock::given(path("/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(version))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = kubeconfig(dir.path(), &server.uri());
    let login = check(&[file], "kind").await;
    assert_eq!(
        login,
        Login::SignedIn {
            version: "v1.33.4".into()
        }
    );
}

#[tokio::test]
async fn a_refused_sign_in_is_tried_once_more_then_said() {
    let server = MockServer::start().await;
    let status = serde_json::json!({"kind": "Status", "code": 401, "message": "Unauthorized"});
    Mock::given(path("/version"))
        .respond_with(ResponseTemplate::new(401).set_body_json(status))
        .expect(2)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = kubeconfig(dir.path(), &server.uri());
    let login = check(&[file], "kind").await;
    assert!(matches!(login, Login::Refused(_)), "{login:?}");
}

#[tokio::test]
async fn a_context_no_file_names_fails_without_a_call() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let file = kubeconfig(dir.path(), "http://127.0.0.1:9");
    let login = check(&[file], "prod").await;
    assert!(matches!(login, Login::Failed(_)), "{login:?}");
}
