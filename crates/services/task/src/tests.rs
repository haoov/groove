//! The sources the config turns on, built once for as long as it stays the same.

use std::sync::Arc;

use crate::State;

fn config(host: &str) -> groove_types::Config {
    serde_json::from_value(serde_json::json!({
        "git": { "worktree_root": "~" },
        "github": {
            "host": host,
            "token": "t",
            "properties": { "status": "Status", "priority": "Priority" },
            "status_map": { "ready": ["Todo"], "in_progress": ["In progress"], "done": ["Done"] },
            "priority_map": { "high": ["P1"], "medium": ["P2"], "low": ["P3"] }
        }
    }))
    .expect("a config")
}

#[test]
fn the_same_config_keeps_the_same_sources_and_a_new_one_builds_them_again() {
    let state = State::default();
    let one = config("https://github.example");
    let first = state.sources(Some(&one));
    assert_eq!(first.len(), 1);
    assert!(Arc::ptr_eq(&first, &state.sources(Some(&one))), "kept");
    let other = config("https://elsewhere.example");
    assert!(
        !Arc::ptr_eq(&first, &state.sources(Some(&other))),
        "built again"
    );
    assert!(state.sources(None).is_empty());
}

#[test]
fn a_template_asked_of_no_source_in_particular_names_the_ones_to_pick_from() {
    let mut both = config("https://github.example");
    both.notion = serde_json::from_value(serde_json::json!({
        "token": "t", "database_id": "DB", "user_id": "U",
        "properties": { "status": "Status" },
        "status_map": { "ready": [], "in_progress": [], "done": [] },
        "priority_map": { "high": [], "medium": [], "low": [] },
        "filters": { "exclude_statuses": [] }
    }))
    .map(Some)
    .expect("a notion config");
    let sources = State::default().sources(Some(&both));
    let said = crate::unpicked(&sources, None);
    assert!(
        said.to_string().contains("notion, github") || said.to_string().contains("github, notion"),
        "{said}"
    );
    let none = crate::unpicked(&[], None);
    assert!(none.to_string().contains("no source is set up"), "{none}");
}
