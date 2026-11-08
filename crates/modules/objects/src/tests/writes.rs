use groove_types::{ErrorKind, KubeKind, Target};
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::watch::kubeconfig;
use crate::{patch, preview};

fn scale() -> Target {
    Target {
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
        subresource: Some("scale".into()),
    }
}

#[tokio::test]
async fn a_patch_lands_on_the_subresource_of_the_named_object() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path(
            "/apis/apps/v1/namespaces/paxone/deployments/api/scale",
        ))
        .and(query_param("dryRun", "All"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "spec": { "replicas": 3 } })),
        )
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [kubeconfig(dir.path(), &server.uri())];
    let body = json!({ "spec": { "replicas": 3 } });
    let object = patch(&paths, &scale(), &body, None, true)
        .await
        .expect("a patched object");
    assert_eq!(object["spec"]["replicas"], 3);
}

#[tokio::test]
async fn a_patch_on_an_object_that_moved_is_a_conflict_naming_it() {
    let server = MockServer::start().await;
    let status = json!({ "kind": "Status", "apiVersion": "v1", "status": "Failure",
        "message": "the object has been modified", "reason": "Conflict", "code": 409 });
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(409).set_body_json(status))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [kubeconfig(dir.path(), &server.uri())];
    let body = json!({ "spec": { "replicas": 3 } });
    let refused = patch(&paths, &scale(), &body, Some("7"), false)
        .await
        .expect_err("a conflict");
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(
        refused.message.contains("paxone/deployments/api/scale"),
        "{}",
        refused.message
    );
}

#[tokio::test]
async fn a_preview_tries_the_patch_on_the_version_read_and_diffs_what_it_changes() {
    let server = MockServer::start().await;
    let at = "/apis/apps/v1/namespaces/paxone/deployments/api/scale";
    let object = |replicas: u32, version: &str| {
        json!({ "metadata": { "name": "api", "resourceVersion": version, "generation": 4,
            "managedFields": [{ "manager": "kubectl" }] }, "spec": { "replicas": replicas } })
    };
    Mock::given(method("GET"))
        .and(path(at))
        .respond_with(ResponseTemplate::new(200).set_body_json(object(1, "7")))
        .mount(&server)
        .await;
    let sent = json!({ "spec": { "replicas": 3 }, "metadata": { "resourceVersion": "7" } });
    Mock::given(method("PATCH"))
        .and(path(at))
        .and(query_param("dryRun", "All"))
        .and(body_json(sent))
        .respond_with(ResponseTemplate::new(200).set_body_json(object(3, "8")))
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = [kubeconfig(dir.path(), &server.uri())];
    let body = json!({ "spec": { "replicas": 3 } });
    let tried = preview(&paths, &scale(), &body).await.expect("a preview");
    assert_eq!(tried.version, "7");
    assert!(
        tried.diff.contains("-  replicas: 1\n+  replicas: 3"),
        "{}",
        tried.diff
    );
    assert!(!tried.diff.contains("resourceVersion"), "{}", tried.diff);
    assert!(!tried.before.contains("managedFields"), "{}", tried.before);
    assert!(tried.after.contains("replicas: 3") && !tried.after.contains("generation"));
}
