use crate::tests::{file, token_context};
use crate::{Auth, contexts};

const EXEC_AND_CERT: &str = "apiVersion: v1
kind: Config
clusters:
- name: hub
  cluster:
    server: https://hub.example:6443
- name: elpis
  cluster:
    server: https://elpis.example:6443
users:
- name: oidc
  user:
    exec:
      apiVersion: client.authentication.k8s.io/v1
      command: kubelogin
      args: [get-token]
- name: admin
  user:
    client-certificate-data: Y2VydA==
    client-key-data: a2V5
contexts:
- name: platform-hub
  context:
    cluster: hub
    user: oidc
- name: platform-elpis
  context:
    cluster: elpis
    user: admin
";

#[tokio::test]
async fn a_context_names_its_server_and_how_it_signs_in() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = file(dir.path(), "config", EXEC_AND_CERT);
    let found = contexts(&[path]).await.expect("contexts read");
    let hub = &found[0];
    assert_eq!(hub.name, "platform-hub");
    assert_eq!(hub.server.as_deref(), Some("https://hub.example:6443"));
    assert_eq!(hub.auth, Auth::Exec("kubelogin".into()));
    assert_eq!(found[1].auth, Auth::Certificate);
}

#[tokio::test]
async fn the_files_merge_in_order_and_a_missing_one_is_skipped() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let first = file(dir.path(), "first", EXEC_AND_CERT);
    let second = file(
        dir.path(),
        "second",
        &token_context("kind", "http://127.0.0.1:1"),
    );
    let missing = dir.path().join("missing");
    let found = contexts(&[first, missing, second])
        .await
        .expect("contexts read");
    let names: Vec<&str> = found.iter().map(|one| one.name.as_str()).collect();
    assert_eq!(names, ["platform-hub", "platform-elpis", "kind"]);
    assert_eq!(found[2].auth, Auth::Token);
    assert_eq!(found[2].namespace.as_deref(), Some("default"));
}

#[tokio::test]
async fn no_file_at_all_is_no_context() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let found = contexts(&[dir.path().join("none")]).await.expect("read");
    assert!(found.is_empty());
}
