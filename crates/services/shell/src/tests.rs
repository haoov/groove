//! A session's terminal tabs: one terminal or several side by side, and which takes the keys.

use groove_types::SessionId;

use crate::{Event, State, apply};

fn session() -> SessionId {
    SessionId::new("s")
}

fn shown(state: &State) -> Vec<u64> {
    let shells = state.shells(&session()).expect("the session's terminals");
    shells.shown().iter().map(|one| one.id).collect()
}

#[test]
fn plus_opens_a_tab_of_its_own_and_split_adds_beside_the_one_up() {
    let mut state = State::default();
    let one = state.reserve(&session(), false);
    let two = state.reserve(&session(), true);
    assert_eq!(shown(&state), [one, two], "one and two share a tab");
    let three = state.reserve(&session(), false);
    assert_eq!(shown(&state), [three], "three stands alone in its own");
    let shells = state.shells(&session()).unwrap();
    assert_eq!(shells.tabs.len(), 2);
    assert_eq!(shells.focused(), Some(three));
}

#[test]
fn three_can_stand_side_by_side_in_one_tab() {
    let mut state = State::default();
    let ids: Vec<u64> = [false, true, true]
        .into_iter()
        .map(|beside| state.reserve(&session(), beside))
        .collect();
    assert_eq!(shown(&state), ids);
}

#[test]
fn focusing_a_pane_moves_the_keys_and_no_pane() {
    let mut state = State::default();
    let one = state.reserve(&session(), false);
    let two = state.reserve(&session(), true);
    state.focus(&session(), one);
    assert_eq!(shown(&state), [one, two], "the panes keep their places");
    assert_eq!(state.shells(&session()).unwrap().focused(), Some(one));
}

#[test]
fn focusing_a_terminal_of_another_tab_brings_that_tab_up() {
    let mut state = State::default();
    let one = state.reserve(&session(), false);
    let two = state.reserve(&session(), false);
    assert_eq!(shown(&state), [two]);
    state.focus(&session(), one);
    assert_eq!(shown(&state), [one]);
}

#[test]
fn closing_a_pane_leaves_its_tab_and_closing_the_last_takes_the_tab() {
    let mut state = State::default();
    let one = state.reserve(&session(), false);
    let two = state.reserve(&session(), true);
    let three = state.reserve(&session(), false);
    assert!(state.close(&session(), two).is_some());
    state.select_tab(&session(), state.shells(&session()).unwrap().tabs[0].id);
    assert_eq!(shown(&state), [one], "the pane beside it keeps the tab");
    assert!(state.close(&session(), one).is_some());
    assert_eq!(
        shown(&state),
        [three],
        "the tab went, the next one stands up"
    );
}

#[test]
fn closing_a_tab_hands_over_every_terminal_in_it() {
    let mut state = State::default();
    state.reserve(&session(), false);
    state.reserve(&session(), true);
    let tab = state.shells(&session()).unwrap().tabs[0].id;
    assert_eq!(state.close_tab(&session(), tab).len(), 2);
    assert!(state.shells(&session()).unwrap().tabs.is_empty());
}

#[test]
fn ending_a_session_hands_over_every_terminal_it_held() {
    let mut state = State::default();
    state.reserve(&session(), false);
    state.reserve(&session(), true);
    state.reserve(&session(), false);
    assert_eq!(state.end(&session()).len(), 3);
    assert!(state.shells(&session()).is_none());
}

#[test]
fn a_terminal_that_exited_keeps_its_place_with_its_code() {
    let mut state = State::default();
    let id = state.reserve(&session(), false);
    let exited = Event::Exited {
        session: session(),
        id,
        code: 3,
    };
    apply(&mut state, exited);
    let shell = state.shells(&session()).unwrap().get(id).unwrap();
    assert_eq!(shell.exited, Some(3));
}
