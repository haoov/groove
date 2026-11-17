use groove_types::{Alert, Error, Timestamp};

use crate::State;

fn alert(key: &str, priority: &str) -> Alert {
    Alert {
        id: key.into(),
        title: key.into(),
        description: String::new(),
        dedup_key: key.into(),
        source_url: None,
        priority: Some(priority.into()),
        cluster: None,
        namespace: None,
        environment: None,
        created_at: Timestamp::new(0),
    }
}

#[test]
fn a_read_is_due_once_none_is_in_flight_and_the_last_has_aged() {
    let mut state = State::default();
    assert!(state.due(Timestamp::new(100), 60), "never read");
    state.sent(Timestamp::new(100));
    assert!(!state.due(Timestamp::new(500), 60), "one in flight");
    state.read(Ok(vec![]), Timestamp::new(101));
    assert!(!state.due(Timestamp::new(150), 60));
    assert!(state.due(Timestamp::new(161), 60));
}

#[test]
fn a_failed_read_keeps_the_last_alerts_and_says_why() {
    let mut state = State::default();
    state.read(
        Ok(vec![alert("a", "Critical"), alert("b", "Warning")]),
        Timestamp::new(1),
    );
    state.read(
        Err(Error::invalid("inc is not logged in")),
        Timestamp::new(2),
    );
    assert_eq!(state.firing.len(), 2);
    assert_eq!(state.failed.as_deref(), Some("inc is not logged in"));
    assert_eq!(state.counts(), [1, 0, 1, 0]);
    assert_eq!(state.find("b").map(|one| one.title.as_str()), Some("b"));
}
