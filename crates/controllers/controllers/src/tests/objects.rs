//! Kinds and watchers: a watcher runs off the loop and its rows land in the store.

use std::time::Duration;

use groove_types::{KubeKind, WatchKey};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::cluster::Command;
use crate::tests::fixture::kubeconfig;
use crate::{Command as Cmd, dispatch};

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
            short_names: Vec::new(),
        },
        namespace: Some("shop".into()),
        selector: None,
    }
}

/// A server that lists one pod and serves the kinds; its watch ends at once.
async fn server() -> MockServer {
    let server = MockServer::start().await;
    let plain = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    let table = serde_json::json!({ "kind": "Table", "metadata": { "resourceVersion": "42" },
        "columnDefinitions": [{ "name": "Name", "priority": 0 }],
        "rows": [{ "cells": ["api-0"], "object": { "metadata": {
            "uid": "u1", "name": "api-0", "namespace": "shop", "resourceVersion": "42" } } }] });
    Mock::given(path("/api/v1/namespaces/shop/pods"))
        .and(query_param("limit", "5000"))
        .respond_with(plain(table))
        .mount(&server)
        .await;
    Mock::given(path("/api/v1/namespaces/shop/pods"))
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
    let names = serde_json::json!({ "kind": "Table", "metadata": { "resourceVersion": "9" },
        "columnDefinitions": [{ "name": "Name", "priority": 0 }],
        "rows": [{ "cells": ["shop"], "object": { "metadata": { "uid": "n1", "name": "shop" } } },
                 { "cells": ["argocd"], "object": { "metadata": { "uid": "n2", "name": "argocd" } } }] });
    Mock::given(path("/api/v1/namespaces"))
        .respond_with(plain(names))
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

#[test]
fn a_context_s_namespaces_are_listed_by_name() {
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
    let list = Command::ListNamespaces {
        context: "kind".into(),
    };
    dispatch(Cmd::Cluster(list), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let names = state.cluster.store.namespaces("kind").expect("listed");
    assert_eq!(names, ["argocd", "shop"]);
}

#[test]
fn a_failed_discovery_is_said_once_and_waits_before_it_is_asked_again() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let server = spawner.block_on(MockServer::start());
    spawner.block_on(
        Mock::given(path("/api"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server),
    );
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
    let discover = || {
        Cmd::Cluster(Command::Discover {
            context: "kind".into(),
            again: false,
        })
    };
    let asked = || spawner.block_on(server.received_requests()).unwrap().len();
    dispatch(discover(), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let once = asked();
    dispatch(discover(), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert_eq!(asked(), once, "no second discovery while it waits");
    assert!(state.cluster.store.discovering("kind"));
    spawner.settle(Duration::from_millis(1200), &mut state, &services);
    assert!(
        !state.cluster.store.discovering("kind"),
        "free to ask again"
    );
    dispatch(discover(), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert!(asked() > once);
    assert_eq!(state.errors.len(), 1, "said once: {:?}", state.errors);
}
