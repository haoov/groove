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

#[test]
fn a_list_is_pinned_once_and_let_go_once() {
    use groove_types::{Attached, KubeKind, Pin};
    let mut state = State::default();
    state.open(session("a"), Timestamp::new(0));
    let kind = KubeKind {
        group: String::new(),
        version: "v1".into(),
        kind: "Pod".into(),
        plural: "pods".into(),
        namespaced: true,
        watchable: true,
        short_names: vec!["po".into()],
    };
    let hub = Attached {
        context: "hub".into(),
        namespace: None,
    };
    let pin = Pin::new(&kind, [hub.clone(), hub]);
    assert_eq!(pin.scope.len(), 1, "each pair once");
    let open = state.get_mut(&SessionId::new("a")).expect("open");
    assert!(open.pin(pin.clone()));
    assert!(!open.pin(pin.clone()), "already pinned");
    assert_eq!(open.pins, std::slice::from_ref(&pin));
    assert!(open.unpin(&pin));
    assert!(!open.unpin(&pin));
    assert!(open.pins.is_empty());
}
