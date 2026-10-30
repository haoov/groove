//! A launch lands only while its session waits on it: an earlier one, or one ended meanwhile, takes no row.

use groove_types::{Error, SessionId, Timestamp};

use crate::State;

fn failed() -> Result<crate::Run, Error> {
    Err(Error::invalid("no terminal in this test"))
}

#[test]
fn of_two_launches_only_the_last_takes_the_row() {
    let (mut state, session) = (State::default(), SessionId::new("s"));
    let first = state.launch(&session);
    let second = state.launch(&session);
    assert!(
        state
            .started((session.clone(), first), failed(), Timestamp::now())
            .is_none()
    );
    assert!(state.agent(&session).is_none(), "the first one is no one's");
    state.started((session.clone(), second), failed(), Timestamp::now());
    assert_eq!(state.agent(&session).map(|one| one.launch), Some(second));
}

#[test]
fn a_launch_ended_before_it_landed_takes_no_row() {
    let (mut state, session) = (State::default(), SessionId::new("s"));
    let launch = state.launch(&session);
    state.end(&session);
    state.started((session.clone(), launch), failed(), Timestamp::now());
    assert!(state.agent(&session).is_none());
}
