use crate::{ClusterConfig, Config, Hue};

#[test]
fn a_context_added_takes_the_first_free_hue() {
    let added = ClusterConfig::new("ovh-hub", &[Hue::Sapphire]);
    assert_eq!(added.hue, Hue::Mauve);
}

#[test]
fn a_file_without_clusters_reads_and_writes_without_them() {
    let config: Config =
        serde_json::from_str(r#"{ "git": { "worktree_root": "~/code" } }"#).expect("a config");
    assert!(config.clusters.is_empty());
    let written = serde_json::to_string(&config).expect("written");
    assert!(!written.contains("clusters"));
}

#[test]
fn a_built_in_kind_stands_under_its_heading_a_crd_under_its_group() {
    use crate::{Builtin, KindHeading, KubeKind};
    let kind = |group: &str, name: &str, namespaced: bool| KubeKind {
        group: group.into(),
        version: "v1".into(),
        kind: name.into(),
        plural: String::new(),
        namespaced,
        watchable: true,
    };
    let builtin = KindHeading::Builtin;
    let cases = [
        (
            kind("apps", "Deployment", true),
            builtin(Builtin::Workloads),
        ),
        (
            kind("networking.k8s.io", "Ingress", true),
            builtin(Builtin::Network),
        ),
        (kind("", "Secret", true), builtin(Builtin::Config)),
        (
            kind("", "PersistentVolumeClaim", true),
            builtin(Builtin::Storage),
        ),
        (
            kind("rbac.authorization.k8s.io", "ClusterRole", false),
            builtin(Builtin::Rbac),
        ),
        (
            kind("coordination.k8s.io", "Lease", true),
            builtin(Builtin::Other),
        ),
        (
            kind("argoproj.io", "Application", true),
            KindHeading::Group("argoproj.io".into()),
        ),
        (
            kind("cert-manager.io", "ClusterIssuer", false),
            KindHeading::Group("cert-manager.io".into()),
        ),
        (kind("", "Node", false), KindHeading::Cluster),
    ];
    for (one, heading) in cases {
        assert_eq!(one.heading(), heading, "{}", one.kind);
    }
    assert!(builtin(Builtin::Other) < KindHeading::Group("a.io".into()));
    assert!(KindHeading::Group("z.io".into()) < KindHeading::Cluster);
}

#[test]
fn a_status_word_and_a_ready_count_say_how_an_object_stands() {
    use crate::{Health, ready_health, status_health};
    assert_eq!(status_health("Running"), Some(Health::Good));
    assert_eq!(status_health("CrashLoopBackOff"), Some(Health::Failing));
    assert_eq!(status_health("Init:Error"), Some(Health::Failing));
    assert_eq!(status_health("Init:0/2"), Some(Health::Waiting));
    assert_eq!(status_health("Completed"), Some(Health::Done));
    assert_eq!(status_health("v1.33.4"), None);
    assert_eq!(ready_health("2/2"), Some(Health::Good));
    assert_eq!(ready_health("1/3"), Some(Health::Waiting));
    assert_eq!(ready_health("0/1"), Some(Health::Failing));
    assert_eq!(ready_health("0/0"), Some(Health::Done));
    assert_eq!(ready_health("ready"), None);
}

#[test]
fn ages_and_counts_order_by_value_and_words_as_text() {
    use crate::compare_cells;
    use std::cmp::Ordering;
    assert_eq!(compare_cells("5m", "2d4h"), Ordering::Less);
    assert_eq!(compare_cells("9", "10"), Ordering::Less);
    assert_eq!(compare_cells("3 (5m ago)", "12"), Ordering::Less);
    assert_eq!(compare_cells("api", "worker"), Ordering::Less);
}

#[test]
fn a_fuzzy_query_keeps_a_name_holding_its_characters_in_their_order() {
    use crate::fuzzy;
    assert!(fuzzy("pythie-cayzn-staging-flair", "pyth-ca-st-fla"));
    assert!(fuzzy("PYTHIE", "pyt"));
    assert!(!fuzzy("pythie-cayzn-staging-flair", "fla-pyth"));
    assert!(fuzzy("anything", ""));
}
