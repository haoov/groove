use groove_types::{Attached, KubeKind, Target};

use crate::writable;

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
