//! The feed as the rail holds it: a new line on top, a closed session's lines gone.

use groove_types::{Session, SessionId, SessionKind, TimelineEvent, TimelineKind, Timestamp};

use crate::{FEED_MAX, State};

fn session(id: &str) -> Session {
    Session {
        id: SessionId::new(id),
        title: id.into(),
        kind: SessionKind::Explorer,
        created_at: Timestamp::new(0),
    }
}

fn line(session: &str, subject: &str) -> TimelineEvent {
    TimelineEvent {
        session: SessionId::new(session),
        at: Timestamp::new(0),
        kind: TimelineKind::Commit,
        subject: subject.into(),
        payload: serde_json::Value::Null,
    }
}

fn subjects(state: &State) -> Vec<&str> {
    state.feed.iter().map(|one| one.subject.as_str()).collect()
}

#[test]
fn a_new_line_stands_on_top_of_the_feed() {
    let mut state = State::default();
    state.open(session("a"), Timestamp::new(0));
    state.logged(line("a", "first"));
    state.logged(line("a", "second"));
    assert_eq!(subjects(&state), ["second", "first"]);
}

#[test]
fn a_session_off_the_rail_adds_nothing_to_the_feed() {
    let mut state = State::default();
    state.logged(line("gone", "said"));
    assert!(state.feed.is_empty());
}

#[test]
fn the_feed_keeps_its_newest_lines_only() {
    let mut state = State::default();
    state.open(session("a"), Timestamp::new(0));
    for at in 0..=FEED_MAX {
        state.logged(line("a", &at.to_string()));
    }
    assert_eq!(state.feed.len(), FEED_MAX);
    assert_eq!(state.feed[0].subject, FEED_MAX.to_string());
}

#[test]
fn closing_a_session_takes_its_lines_off_the_feed() {
    let mut state = State::default();
    state.open(session("a"), Timestamp::new(0));
    state.open(session("b"), Timestamp::new(0));
    state.logged(line("a", "of a"));
    state.logged(line("b", "of b"));
    state.close(&SessionId::new("a"));
    assert_eq!(subjects(&state), ["of b"]);
}
