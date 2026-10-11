use std::time::Duration;

use groove_kube::Error;

use crate::retry::Backoff;

fn unreachable() -> Error {
    Error::Unreachable {
        context: "kind".into(),
        detail: "reset".into(),
    }
}

#[test]
fn the_wait_doubles_while_a_stream_fails_and_starts_over_once_it_opens() {
    let mut backoff = Backoff::new();
    let waits: Vec<u64> = (0..7)
        .map(|_| backoff.after(&unreachable()).as_secs())
        .collect();
    assert_eq!(waits, [1, 2, 4, 8, 16, 30, 30]);
    backoff.reset();
    assert_eq!(backoff.after(&unreachable()), Duration::from_secs(1));
}

#[test]
fn an_error_a_retry_does_not_fix_parks_the_stream() {
    let mut backoff = Backoff::new();
    let gone = Error::Api {
        context: "kind".into(),
        status: 404,
        message: "the server could not find the requested resource".into(),
    };
    assert_eq!(backoff.after(&gone), Duration::from_secs(300));
    assert_eq!(backoff.after(&unreachable()), Duration::from_secs(1));
}
