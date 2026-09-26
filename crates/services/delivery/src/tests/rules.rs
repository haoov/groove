//! What an MR write is refused before it is sent, per worktree.

use groove_types::{Forge, Mr, MrId, MrState, WorktreeId};

use crate::{MrAct, State};

fn holding(state: &mut State, worktree: &str, mr: MrState) {
    state.remembered(vec![Mr {
        id: MrId::new(format!("m-{worktree}")),
        worktree: WorktreeId::new(worktree),
        forge: Forge::Github,
        remote_id: "7".into(),
        url: "https://example.test/pull/7".into(),
        state: mr,
    }]);
}

#[test]
fn a_worktree_with_an_open_mr_is_offered_no_second_one() {
    let mut state = State::default();
    holding(&mut state, "w1", MrState::Open);
    let w1 = WorktreeId::new("w1");
    assert!(state.allows(&w1, MrAct::Open).is_err());
    assert!(state.allows(&w1, MrAct::Edit).is_ok());
    assert!(state.allows(&w1, MrAct::Close).is_ok());
}

#[test]
fn an_mr_that_is_not_open_cannot_be_written_or_closed() {
    let mut state = State::default();
    holding(&mut state, "w1", MrState::Merged);
    let w1 = WorktreeId::new("w1");
    assert!(state.allows(&w1, MrAct::Edit).is_err());
    assert!(state.allows(&w1, MrAct::Close).is_err());
}

#[test]
fn each_worktree_answers_for_its_own_mr() {
    let mut state = State::default();
    holding(&mut state, "w1", MrState::Open);
    let w2 = WorktreeId::new("w2");
    assert!(
        state.allows(&w2, MrAct::Open).is_ok(),
        "w1's mr is not w2's"
    );
    assert!(state.allows(&w2, MrAct::Edit).is_err());
}

#[test]
fn nothing_is_written_while_a_read_of_it_is_out() {
    let mut state = State::default();
    holding(&mut state, "w1", MrState::Open);
    let w1 = WorktreeId::new("w1");
    state.poll.sent(&w1);
    assert!(state.allows(&w1, MrAct::Edit).is_err());
}
