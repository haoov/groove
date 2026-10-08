use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use groove_types::{KubeKind, WatchKey};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::{Batch, Delta, Stop, watch};

pub(crate) fn kubeconfig(dir: &Path, server: &str) -> PathBuf {
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

fn pods() -> WatchKey {
    WatchKey {
        context: "kind".into(),
        kind: KubeKind {
            group: String::new(),
            version: "v1".into(),
            kind: "Pod".into(),
            plural: "pods".into(),
            namespaced: true,
            watchable: true,
        },
        namespace: Some("paxone".into()),
        selector: None,
    }
}

fn table(rows: &[(&str, &str)], version: &str) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = rows
        .iter()
        .map(|(uid, name)| {
            serde_json::json!({ "cells": [name], "object": { "metadata": {
            "uid": uid, "name": name, "namespace": "paxone", "resourceVersion": version } } })
        })
        .collect();
    serde_json::json!({ "kind": "Table", "metadata": { "resourceVersion": version },
        "columnDefinitions": [{ "name": "Name", "priority": 0 }], "rows": rows })
}

/// Runs the watcher for a moment and returns every batch it sent.
async fn batches(server: &MockServer) -> Vec<Batch> {
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), &server.uri())];
    let sent = Arc::new(Mutex::new(Vec::new()));
    let held = sent.clone();
    let stop = Stop::new();
    let running = tokio::spawn(watch(paths, pods(), &stop, move |batch| {
        held.lock().expect("the batches").push(batch);
    }));
    tokio::time::sleep(Duration::from_millis(300)).await;
    stop.stop();
    running.await.expect("the watcher ended");
    sent.lock().expect("the batches").clone()
}

#[tokio::test]
async fn the_list_comes_first_whole_then_the_watch_s_changes_from_its_version() {
    let server = MockServer::start().await;
    let list = table(&[("u1", "api-0")], "42");
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("limit", "5000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(list))
        .mount(&server)
        .await;
    let events = [
        serde_json::json!({ "type": "ADDED", "object": table(&[("u2", "api-1")], "43") }),
        serde_json::json!({ "type": "DELETED", "object": table(&[("u1", "api-0")], "44") }),
    ];
    let body: String = events.iter().map(|one| format!("{one}\n")).collect();
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("watch", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .mount(&server)
        .await;
    let sent = batches(&server).await;
    let Batch::Reset { columns, rows } = &sent[0] else {
        panic!("the list comes first: {sent:?}");
    };
    assert_eq!((columns.len(), rows[0].name.as_str()), (1, "api-0"));
    assert_eq!(sent[1], Batch::Watching, "then the watch opens");
    let Batch::Changes(changes) = &sent[2] else {
        panic!("then its changes: {sent:?}");
    };
    let put = changes
        .iter()
        .position(|one| matches!(one, Delta::Put(row) if row.name == "api-1"));
    let gone = changes
        .iter()
        .position(|one| matches!(one, Delta::Gone(row) if row.uid == "u1"));
    assert!(
        put.zip(gone).is_some_and(|(put, gone)| put < gone),
        "{changes:?}"
    );
    let asked = server.received_requests().await.expect("recorded");
    let first_watch = asked
        .iter()
        .find(|one| one.url.query().is_some_and(|q| q.contains("watch=1")));
    let from = first_watch.expect("a watch").url.query().expect("a query");
    assert!(from.contains("resourceVersion=42"), "{from}");
}

#[tokio::test]
async fn an_expired_version_lists_again_from_the_start() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("limit", "5000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(table(&[("u1", "api-0")], "42")))
        .mount(&server)
        .await;
    let gone = serde_json::json!({ "kind": "Status", "code": 410, "message": "too old" });
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("watch", "1"))
        .respond_with(ResponseTemplate::new(410).set_body_json(gone))
        .mount(&server)
        .await;
    let sent = batches(&server).await;
    let resets = sent
        .iter()
        .filter(|one| matches!(one, Batch::Reset { .. }))
        .count();
    assert!(resets >= 2, "{sent:?}");
    assert!(
        !sent.iter().any(|one| matches!(one, Batch::Failed(_))),
        "{sent:?}"
    );
}

#[tokio::test]
async fn a_first_list_shows_its_first_page_at_once_and_adds_the_next_as_they_come() {
    let server = MockServer::start().await;
    let mut first = table(&[("u1", "api-0")], "42");
    first["metadata"]["continue"] = serde_json::json!("page-2");
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("resourceVersion", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(first))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("continue", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(table(&[("u2", "api-1")], "42")))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("watch", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;
    let sent = batches(&server).await;
    assert!(
        matches!(&sent[0], Batch::Reset { rows, .. } if rows.len() == 1),
        "{sent:?}"
    );
    assert!(
        matches!(&sent[1], Batch::Changes(more) if more.len() == 1),
        "{sent:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn changes_go_out_16ms_after_the_first_and_never_twice_within_250ms() {
    use groove_kube::{Change, Row};
    use tokio::time::{Instant, sleep};
    let put = |name: &str| {
        Change::Put(Row {
            uid: name.into(),
            name: name.into(),
            namespace: None,
            version: "1".into(),
            created: None,
            cells: vec![name.into()],
        })
    };
    let steps = vec![
        (0, vec![put("a"), put("b")]),
        (100, vec![put("c")]),
        (600, vec![put("d")]),
        (1_300, Vec::new()),
    ];
    let stream = futures_util::stream::unfold(steps.into_iter(), |mut steps| async move {
        let (wait, changes) = steps.next()?;
        sleep(Duration::from_millis(wait)).await;
        Some((Ok(changes), steps))
    });
    let started = Instant::now();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let into = sent.clone();
    let send = move |batch: Batch| {
        if let Batch::Changes(deltas) = batch {
            let at = started.elapsed().as_millis();
            into.lock().expect("the batches").push((at, deltas.len()));
        }
    };
    let (mut version, mut columns) = (String::new(), Vec::new());
    crate::watch::watched(stream, (&mut version, &mut columns), &send)
        .await
        .expect("the watch");
    let sent = sent.lock().expect("the batches").clone();
    assert_eq!(sent, [(16, 2), (266, 1), (716, 1)]);
}
