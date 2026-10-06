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
