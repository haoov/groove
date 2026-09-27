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
