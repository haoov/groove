use groove_types::{Attached, KubeKind, Target};

use crate::Asked;
use crate::writable;
use crate::writes::target;

fn api(namespace: Option<&str>) -> Target {
    Target {
        context: "prod".into(),
        kind: KubeKind {
            group: "apps".into(),
            version: "v1".into(),
            kind: "Deployment".into(),
            plural: "deployments".into(),
            namespaced: namespace.is_some(),
            watchable: true,
            short_names: Vec::new(),
        },
        namespace: namespace.map(str::to_string),
        name: "api".into(),
        subresource: None,
    }
}

fn held(context: &str, namespace: Option<&str>) -> Attached {
    Attached {
        context: context.into(),
        namespace: namespace.map(str::to_string),
    }
}

#[test]
fn a_read_only_context_refuses_a_write_the_session_holds() {
    let refused = writable(&api(Some("paxone")), true, &[held("prod", None)]);
    assert!(refused.is_err_and(|e| e.message.contains("read-only")));
}

#[test]
fn a_write_lands_only_on_a_context_the_session_holds() {
    let refused = writable(&api(Some("paxone")), false, &[held("staging", None)]);
    assert!(refused.is_err_and(|e| e.message.contains("no context prod")));
}

#[test]
fn a_session_on_one_namespace_writes_there_and_nowhere_else() {
    let on = [held("prod", Some("paxone"))];
    assert!(writable(&api(Some("paxone")), false, &on).is_ok());
    let elsewhere = writable(&api(Some("cayzn")), false, &on);
    assert!(elsewhere.is_err_and(|e| e.message.contains("namespace cayzn")));
    let unscoped = writable(&api(None), false, &on);
    assert!(unscoped.is_err_and(|e| e.message.contains("the whole of prod")));
}

#[test]
fn a_session_on_the_whole_cluster_writes_in_any_namespace_and_outside_them() {
    let on = [held("prod", None)];
    assert!(writable(&api(Some("cayzn")), false, &on).is_ok());
    assert!(writable(&api(None), false, &on).is_ok());
}

fn kind(group: &str, plural: &str, name: &str, namespaced: bool) -> KubeKind {
    KubeKind {
        group: group.into(),
        version: "v1".into(),
        kind: name.into(),
        plural: plural.into(),
        namespaced,
        watchable: true,
        short_names: Vec::new(),
    }
}

fn asked(kind: &str, namespace: Option<&str>) -> Asked {
    Asked {
        context: "prod".into(),
        kind: kind.into(),
        namespace: namespace.map(str::to_string),
        name: "api".into(),
        subresource: None,
    }
}

#[test]
fn a_kind_is_named_by_its_plural_its_name_or_its_plural_and_group() {
    let kinds = [
        kind("apps", "deployments", "Deployment", true),
        kind("", "nodes", "Node", false),
    ];
    for named in ["deployments", "Deployment", "deployments.apps"] {
        let found = target(&kinds, asked(named, Some("paxone"))).expect(named);
        assert_eq!(found.kind.group, "apps", "{named}");
    }
    let node = target(&kinds, asked("nodes", Some("paxone"))).expect("a node");
    assert_eq!(node.namespace, None, "a node stands in no namespace");
    let unnamed = target(&kinds, asked("deployments", None));
    assert!(unnamed.is_err_and(|e| e.message.contains("name it")));
}

#[test]
fn a_kind_two_groups_serve_must_name_its_group() {
    let kinds = [
        kind("networking.k8s.io", "ingresses", "Ingress", true),
        kind("extensions", "ingresses", "Ingress", true),
    ];
    let both = target(&kinds, asked("ingresses", Some("paxone")));
    assert!(both.is_err_and(|e| e.message.contains("ingresses.extensions")));
    let one = target(&kinds, asked("ingresses.extensions", Some("paxone")));
    assert!(one.is_ok());
}

#[test]
fn a_restart_stamps_the_pod_template_of_a_workload_and_refuses_a_pod() {
    let at = groove_types::Timestamp::new(1_788_886_029);
    let deployment = kind("apps", "deployments", "Deployment", true);
    let body = crate::restart(&deployment, at).expect("a deployment restarts");
    let stamp = "/spec/template/metadata/annotations/kubectl.kubernetes.io~1restartedAt";
    let stamped = body.pointer(stamp).and_then(|one| one.as_str());
    assert_eq!(stamped, Some("2026-09-08T16:47:09Z"));
    let pod = kind("", "pods", "Pod", true);
    assert!(crate::restart(&pod, at).is_err());
}

#[test]
fn a_scale_patches_the_scale_subresource_of_a_deployment_and_refuses_a_daemonset() {
    let mut api = api(Some("paxone"));
    let body = crate::scale(&mut api, 3).expect("a deployment scales");
    assert_eq!(body.pointer("/spec/replicas"), Some(&serde_json::json!(3)));
    assert_eq!(api.subresource.as_deref(), Some("scale"));
    let mut daemons = api.clone();
    daemons.kind = kind("apps", "daemonsets", "DaemonSet", true);
    assert!(crate::scale(&mut daemons, 3).is_err());
}
