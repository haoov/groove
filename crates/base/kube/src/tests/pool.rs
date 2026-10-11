use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::tests::file;
use crate::{Client, Error};

/// A context named `name` on `server`, its token from a plugin that counts its runs in `runs`.
fn plugged(dir: &Path, name: &str, server: &str) -> PathBuf {
    let plugin = dir.join("plugin.sh");
    let runs = dir.join("runs");
    let script = format!(
        "#!/bin/sh\necho run >> '{}'\necho '{{\"apiVersion\":\"client.authentication.k8s.io/v1\",\"kind\":\"ExecCredential\",\"status\":{{\"token\":\"t\"}}}}'\n",
        runs.display()
    );
    std::fs::write(&plugin, script).expect("a plugin written");
    std::fs::set_permissions(&plugin, std::fs::Permissions::from_mode(0o755)).expect("runnable");
    let yaml = format!(
        "apiVersion: v1
kind: Config
clusters:
- {{name: {name}, cluster: {{server: '{server}'}}}}
users:
- name: {name}
  user:
    exec: {{apiVersion: client.authentication.k8s.io/v1, command: '{}', interactiveMode: Never}}
contexts:
- {{name: {name}, context: {{cluster: {name}, user: {name}}}}}
",
        plugin.display()
    );
    file(dir, "config", &yaml)
}

fn runs(dir: &Path) -> usize {
    let read = std::fs::read_to_string(dir.join("runs")).unwrap_or_default();
    read.lines().count()
}

async fn answering(status: u16) -> MockServer {
    let server = MockServer::start().await;
    let version = serde_json::json!({ "major": "1", "minor": "33", "gitVersion": "v1.33.4",
        "gitCommit": "", "gitTreeState": "clean", "buildDate": "", "goVersion": "go1.24",
        "compiler": "gc", "platform": "linux/amd64",
        "kind": "Status", "message": "Unauthorized", "code": status });
    Mock::given(path("/version"))
        .respond_with(ResponseTemplate::new(status).set_body_json(version))
        .mount(&server)
        .await;
    server
}

async fn asked(paths: &[PathBuf], context: &str) -> crate::Result<String> {
    Client::shared(paths, context).await?.version().await
}

#[tokio::test]
async fn requests_on_one_context_share_its_client_and_sign_in_once() {
    let server = answering(200).await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [plugged(dir.path(), "pooled", &server.uri())];
    asked(&paths, "pooled").await.expect("a version");
    let once = runs(dir.path());
    for _ in 0..3 {
        asked(&paths, "pooled").await.expect("a version");
    }
    assert_eq!(runs(dir.path()), once);
}

#[tokio::test]
async fn a_refused_sign_in_drops_the_client_and_the_next_request_signs_in_again() {
    let server = answering(401).await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [plugged(dir.path(), "refusing", &server.uri())];
    let refused = asked(&paths, "refusing").await;
    assert!(matches!(refused, Err(Error::Refused { .. })), "{refused:?}");
    let once = runs(dir.path());
    let _ = asked(&paths, "refusing").await;
    assert_eq!(runs(dir.path()), 2 * once);
}

#[tokio::test]
async fn a_kubeconfig_changed_makes_a_new_client() {
    let server = answering(200).await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [plugged(dir.path(), "rewritten", &server.uri())];
    asked(&paths, "rewritten").await.expect("a version");
    let once = runs(dir.path());
    let longer = std::fs::read_to_string(&paths[0]).expect("the file") + "preferences: {}\n";
    std::fs::write(&paths[0], longer).expect("the file rewritten");
    asked(&paths, "rewritten").await.expect("a version");
    assert_eq!(runs(dir.path()), 2 * once);
}
