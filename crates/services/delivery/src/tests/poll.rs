//! Which worktrees a tick reads: focus, the interval, and what is already out.

use groove_types::{Forge, Mr, MrId, MrState, Timestamp, WorktreeId};

use crate::State;

/// The interval the config asks for, in seconds.
const EVERY: i64 = 60;

fn now() -> Timestamp {
    Timestamp::new(1_800_000_000)
}

fn later() -> Timestamp {
    Timestamp::new(now().seconds() + EVERY)
}

fn id() -> WorktreeId {
    WorktreeId::new("w1")
}

/// The worktree holds an MR in this state, asked about and answered once already.
fn holding(state: MrState) -> State {
    let mut held = State::default();
    held.remembered(vec![Mr {
        id: MrId::new("m1"),
        worktree: id(),
        forge: Forge::Github,
        remote_id: "7".into(),
        url: "https://example.com/mr/7".into(),
        state,
    }]);
    held.poll.sent(&id());
    held.poll.answered(&id());
    held.poll.ran(now());
    held
}

fn wanted(state: &State, focused: bool, at: Timestamp) -> Vec<WorktreeId> {
    state.wanted(focused, &[id()], &[id()], (at, EVERY))
}

#[test]
fn the_selected_worktree_is_asked_about_once_while_the_window_is_focused() {
    let mut state = State::default();
    assert!(wanted(&state, false, now()).is_empty(), "nobody is looking");
    assert_eq!(wanted(&state, true, now()), [id()], "never asked");
    state.poll.sent(&id());
    state.poll.answered(&id());
    assert!(
        wanted(&state, true, now()).is_empty(),
        "no mr: not asked again"
    );
}

#[test]
fn an_open_mr_is_read_again_once_the_interval_has_passed() {
    let state = holding(MrState::Open);
    assert!(wanted(&state, true, now()).is_empty(), "the pass just ran");
    assert_eq!(wanted(&state, true, later()), [id()]);
}

#[test]
fn a_merged_or_closed_mr_is_left_alone() {
    for settled in [MrState::Merged, MrState::Closed] {
        assert!(wanted(&holding(settled), true, later()).is_empty());
    }
}

#[test]
fn a_read_still_out_is_not_sent_a_second_time() {
    let mut state = holding(MrState::Open);
    state.poll.sent(&id());
    assert!(wanted(&state, true, later()).is_empty(), "one read is out");
    state.poll.answered(&id());
    assert_eq!(wanted(&state, true, later()), [id()]);
}

#[test]
fn the_window_coming_back_asks_about_everything_again() {
    let mut state = holding(MrState::Merged);
    assert!(wanted(&state, true, now()).is_empty());
    state.poll.woke();
    assert_eq!(
        wanted(&state, true, now()),
        [id()],
        "asked about once on focus"
    );
}

#[test]
fn a_worktree_wanted_twice_is_read_once() {
    let mut state = holding(MrState::Open);
    state.poll.woke();
    assert_eq!(wanted(&state, true, later()), [id()]);
}

#[test]
fn a_worktree_off_the_rail_is_not_polled() {
    let state = holding(MrState::Open);
    assert!(state.wanted(true, &[], &[], (later(), EVERY)).is_empty());
    assert!(!state.polls(true, &[]));
    assert!(state.polls(true, &[id()]));
}

#[test]
fn every_worktree_of_the_open_session_is_asked_about_once() {
    let (one, two) = (WorktreeId::new("w1"), WorktreeId::new("w2"));
    let shown = [one.clone(), two.clone()];
    let mut state = State::default();
    assert_eq!(state.wanted(true, &shown, &shown, (now(), EVERY)), shown);
    state.poll.sent(&one);
    state.poll.answered(&one);
    assert_eq!(state.wanted(true, &shown, &shown, (now(), EVERY)), [two]);
}
