use groove_types::ApprovalId;

use crate::{State, prepare};

fn change() -> crate::Prepared {
    let pair = (
        "a.yaml".to_string(),
        "replicas: 1\n".to_string(),
        "replicas: 2\n".to_string(),
    );
    prepare(vec![pair])
}

#[test]
fn a_review_takes_the_change_s_place_until_its_own_write_is_decided() {
    let mut state = State::default();
    state.review(&ApprovalId::new("one"), change());
    assert_eq!(state.reviewed(), Some("one"));
    assert!(state.readonly() && !state.changes.is_empty());
    assert!(
        !state.decided(&ApprovalId::new("two")),
        "another write leaves it"
    );
    assert!(state.decided(&ApprovalId::new("one")));
    assert_eq!(state.reviewed(), None);
    assert!(
        state.changes.is_empty(),
        "the worktree's change reads again"
    );
}
