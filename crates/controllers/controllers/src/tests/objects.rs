//! Kinds and watchers: a watcher runs off the loop and its rows land in the store.

use std::time::Duration;

use groove_types::{KubeKind, WatchKey};
use wiremock::matchers::{path, query_param};
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
    }
}

/// A server that lists one pod and serves the kinds; its watch ends at once.
async fn server() -> MockServer {
    let server = MockServer::start().await;
    let plain = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    let table = serde_json::json!({ "kind": "Table", "metadata": { "resourceVersion": "42" },
        "columnDefinitions": [{ "name": "Name", "priority": 0 }],
        "rows": [{ "cells": ["api-0"], "object": { "metadata": {
            "uid": "u1", "name": "api-0", "namespace": "paxone", "resourceVersion": "42" } } }] });
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("limit", "500"))
        .respond_with(plain(table))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/namespaces/paxone/pods"))
        .and(query_param("watch", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;
    let versions = serde_json::json!({ "kind": "APIVersions", "versions": ["v1"], "serverAddressByClientCIDRs": [] });
    let resources = serde_json::json!({ "kind": "APIResourceList", "groupVersion": "v1", "resources": [
        { "name": "pods", "singularName": "pod", "namespaced": true, "kind": "Pod", "verbs": ["list", "watch"] }
    ] });
    Mock::given(path("/api"))
        .respond_with(plain(versions))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1"))
        .respond_with(plain(resources))
        .mount(&server)
        .await;
    let groups = serde_json::json!({ "kind": "APIGroupList", "groups": [] });
    Mock::given(path("/apis"))
        .respond_with(plain(groups))
        .mount(&server)
        .await;
    server
}

#[test]
fn a_watcher_runs_off_the_loop_and_its_rows_land_in_the_store_for_a_known_context_only() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let server = spawner.block_on(server());
    kubeconfig(home.path(), &server.uri());
    let watch = || {
        Cmd::Cluster(Command::Watch {
            reader: "list".into(),
            key: pods(),
        })
    };
    dispatch(watch(), &mut state, &services, &spawner);
    assert_eq!(state.errors.len(), 1, "kind is not added to Groove yet");
    dispatch(
        Cmd::Cluster(Command::ScanContexts),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    let add = crate::config::Command::AddCluster {
        context: "kind".into(),
    };
    dispatch(Cmd::Config(add), &mut state, &services, &spawner);
    dispatch(watch(), &mut state, &services, &spawner);
    spawner.settle(Duration::from_millis(300), &mut state, &services);
    let held = state.cluster.store.watched(&pods()).expect("watched");
    assert!(held.synced);
    assert_eq!(held.rows[0].name, "api-0");
    let release = Cmd::Cluster(Command::Release {
        reader: "list".into(),
    });
    dispatch(release, &mut state, &services, &spawner);
    assert!(
        state.cluster.store.watched(&pods()).is_some(),
        "an unread watcher runs a while longer"
    );
}

#[test]
fn a_context_s_kinds_are_discovered_and_cached() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let server = spawner.block_on(server());
    kubeconfig(home.path(), &server.uri());
    dispatch(
        Cmd::Cluster(Command::ScanContexts),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    let add = crate::config::Command::AddCluster {
        context: "kind".into(),
    };
    dispatch(Cmd::Config(add), &mut state, &services, &spawner);
    let discover = Command::Discover {
        context: "kind".into(),
        again: false,
    };
    dispatch(Cmd::Cluster(discover), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let kinds = state.cluster.store.kinds("kind").expect("discovered");
    assert_eq!(kinds[0].plural, "pods");
    assert!(state.env.cache_dir.join("kube/kind.json").exists());
}
