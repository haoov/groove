use groove_types::ApprovalId;

use crate::{State, prepare};

fn key() -> (String, String, String) {
    ("hub".into(), "argocd".into(), "shop".into())
}

fn pairs() -> Vec<(String, String, String)> {
    let path = "argo/shop/Deployment/web.yaml".to_string();
    vec![(path, "replicas: 2\n".into(), "replicas: 3\n".into())]
}

#[test]
fn a_drift_shown_takes_the_place_of_the_change_and_nothing_writes_to_it() {
    let mut state = State::default();
    assert!(state.drifted((key(), "7".into()), prepare(pairs())));
    assert!(state.readonly());
    assert_eq!(state.files.len(), 1);
    assert!(!state.changes.is_empty());
    assert_eq!(state.drift(), Some((&key(), "7")));
    assert!(!state.stand_in.as_ref().expect("a drift").empty());
    let coloured = state.coloured.contains_key("argo/shop/Deployment/web.yaml");
    assert!(coloured, "both sides come coloured with the drift");
}

#[test]
fn leaving_the_drift_empties_the_change_for_the_worktree_to_read_again() {
    let mut state = State::default();
    state.drifted((key(), "7".into()), prepare(pairs()));
    assert!(state.undrift());
    assert!(!state.readonly());
    assert!(state.changes.is_empty() && state.files.is_empty());
    assert!(!state.undrift(), "nothing to leave twice");
}

#[test]
fn an_application_with_nothing_differing_is_in_sync() {
    let mut state = State::default();
    state.drifted((key(), "7".into()), prepare(Vec::new()));
    assert!(state.stand_in.as_ref().expect("a drift").empty());
}

#[test]
fn a_cluster_write_waiting_keeps_its_place_from_a_drift() {
    let mut state = State::default();
    state.drifted((key(), "7".into()), prepare(pairs()));
    state.review(&ApprovalId::new("one"), prepare(pairs()));
    assert_eq!(state.drift(), None, "the write takes the drift's place");
    assert!(!state.drifted((key(), "8".into()), prepare(pairs())));
    assert!(!state.undrift(), "leaving the drift keeps the write");
    assert_eq!(state.reviewed(), Some("one"));
}
