use std::sync::{Arc, Mutex};
use std::time::Duration;

use groove_types::{FollowKey, KubeKind};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::watch::kubeconfig;
use crate::{Change, Followed, Stop, follow};

fn pods() -> KubeKind {
    KubeKind {
        group: String::new(),
        version: "v1".into(),
        kind: "Pod".into(),
        plural: "pods".into(),
        namespaced: true,
        watchable: true,
    }
}

fn pod(uid: &str, phase: &str) -> serde_json::Value {
    serde_json::json!({ "metadata": { "name": "api-0", "uid": uid, "namespace": "paxone",
        "resourceVersion": "10" }, "status": { "phase": phase } })
}

#[tokio::test]
async fn one_object_is_listed_by_name_then_its_changes_follow() {
    let server = MockServer::start().await;
    let listed = serde_json::json!({ "metadata": { "resourceVersion": "9" }, "items": [pod("u1", "Pending")] });
    let changed = format!(
        "{}\n",
        serde_json::json!({ "type": "MODIFIED", "object": pod("u1", "Running") })
    );
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("fieldSelector", "metadata.name=api-0"))
        .and(query_param("watch", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(changed))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("fieldSelector", "metadata.name=api-0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(listed))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), &server.uri())];
    let sent = Arc::new(Mutex::new(Vec::new()));
    let held = sent.clone();
    let stop = Stop::new();
    let key = FollowKey::named("kind", &pods(), Some("paxone"), "api-0");
    let running = tokio::spawn(follow(paths, key, &stop, move |batch| {
        held.lock().expect("the batches").push(batch);
    }));
    tokio::time::sleep(Duration::from_millis(300)).await;
    stop.stop();
    running.await.expect("the follower ended");
    let sent = sent.lock().expect("the batches").clone();
    let phase = |one: &groove_types::Described| one.pod.as_ref().map(|part| part.status.clone());
    assert!(
        matches!(&sent[0], Followed::Reset(all) if phase(&all[0]).as_deref() == Some("Pending"))
    );
    assert_eq!(sent[1], Followed::Watching);
    assert!(matches!(&sent[2], Followed::Changes(changes)
        if matches!(&changes[0], Change::Put(one) if phase(one).as_deref() == Some("Running"))));
}

#[tokio::test]
async fn usage_reads_none_without_metrics_server_and_helm_its_latest_revision() {
    let server = MockServer::start().await;
    Mock::given(path(
        "/apis/metrics.k8s.io/v1beta1/namespaces/paxone/pods/api-0",
    ))
    .respond_with(
        ResponseTemplate::new(404)
            .set_body_json(serde_json::json!({ "kind": "Status", "code": 404 })),
    )
    .mount(&server)
    .await;
    let secrets = serde_json::json!({ "items": [
        { "metadata": { "labels": { "version": "36" } } },
        { "metadata": { "labels": { "version": "37" } } }] });
    Mock::given(path("/api/v1/namespaces/paxone/secrets"))
        .and(query_param("labelSelector", "owner=helm,name=paxone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(secrets))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), &server.uri())];
    let usage = crate::usage(&paths, "kind", "paxone", "api-0")
        .await
        .expect("a read");
    assert_eq!(usage, None);
    let revision = crate::helm_revision(&paths, "kind", "paxone", "paxone")
        .await
        .expect("a read");
    assert_eq!(revision, Some(37));
}

#[tokio::test]
async fn usage_reads_each_container_in_cores_and_bytes() {
    let server = MockServer::start().await;
    let read = serde_json::json!({ "containers": [{ "name": "worker", "usage": { "cpu": "180m", "memory": "700Mi" } }] });
    Mock::given(path(
        "/apis/metrics.k8s.io/v1beta1/namespaces/paxone/pods/api-0",
    ))
    .respond_with(ResponseTemplate::new(200).set_body_json(read))
    .mount(&server)
    .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), &server.uri())];
    let usage = crate::usage(&paths, "kind", "paxone", "api-0")
        .await
        .expect("a read");
    let usage = usage.expect("usage");
    assert_eq!(
        usage.containers,
        [("worker".to_string(), 0.18, 734_003_200.0)]
    );
}
