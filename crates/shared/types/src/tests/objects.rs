//! The rules an object's tab reads: where an Application syncs to, what installed an object, what picks it.

use crate::{Described, Destination, server_of};

fn object(namespace: &str, labels: &[(&str, &str)], annotations: &[(&str, &str)]) -> Described {
    let pairs = |all: &[(&str, &str)]| {
        all.iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    };
    Described {
        uid: "u".into(),
        name: "one".into(),
        namespace: Some(namespace.into()),
        labels: pairs(labels),
        annotations: pairs(annotations),
        created: None,
        owners: Vec::new(),
        replicas: None,
        selector: Vec::new(),
        pod: None,
        app: None,
        event: None,
        yaml: "".into(),
    }
}

#[test]
fn a_server_reads_one_way_however_it_is_written() {
    assert_eq!(
        server_of("HTTPS://Kube.Example.com:443/"),
        "https://kube.example.com"
    );
    assert_eq!(
        server_of("http://h:80/k8s/Clusters/c1"),
        "http://h/k8s/Clusters/c1"
    );
    let own = |server: Option<&str>, name: Option<&str>| Destination {
        server: server.map(String::from),
        name: name.map(String::from),
        namespace: None,
    };
    assert!(own(Some("https://kubernetes.default.svc:443"), None).in_cluster());
    assert!(own(None, Some("in-cluster")).in_cluster());
    assert!(!own(Some("https://10.0.0.1:6443"), None).in_cluster());
}

#[test]
fn a_release_is_read_from_the_top_owner_first_then_the_object() {
    let pod = object("app", &[], &[]);
    let top = object(
        "app",
        &[],
        &[
            ("meta.helm.sh/release-name", "api"),
            ("meta.helm.sh/release-namespace", "deploy"),
        ],
    );
    assert_eq!(
        pod.helm_release(Some(&top)),
        Some(("deploy".into(), "api".into()))
    );
    let labelled = object(
        "app",
        &[
            ("app.kubernetes.io/managed-by", "Helm"),
            ("app.kubernetes.io/instance", "web"),
        ],
        &[],
    );
    assert_eq!(
        labelled.helm_release(None),
        Some(("app".into(), "web".into()))
    );
    assert_eq!(pod.helm_release(None), None);
}

#[test]
fn a_service_picks_what_holds_every_label_of_its_selector() {
    let mut service = object("app", &[], &[]);
    let pod = object("app", &[("app", "api"), ("tier", "web")], &[]);
    assert!(!service.selects(&pod), "no selector picks nothing");
    service.selector = vec![("app".into(), "api".into())];
    assert!(service.selects(&pod));
    service.selector.push(("tier".into(), "db".into()));
    assert!(!service.selects(&pod));
}
