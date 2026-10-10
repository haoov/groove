use groove_types::{LogKey, LogLine, LogRange, LogSource};

use crate::{CAP, Logged, Logs, Streaming};

fn worker() -> LogKey {
    LogKey {
        context: "staging".into(),
        namespace: "paxone".into(),
        pod: "api-0".into(),
        source: LogSource::Container("worker".into()),
        previous: false,
        range: LogRange::Since(900),
    }
}

fn line(at: u32, text: &str) -> LogLine {
    LogLine::read(&format!("2026-10-09T14:02:{at:02}.5Z {text}"), None)
}

#[test]
fn lines_land_in_the_text_each_after_its_clock() {
    let mut logs = Logs::default();
    let key = worker();
    assert!(logs.lease(&key, "resource").is_some(), "a stream to open");
    assert!(logs.lease(&key, "agent").is_none(), "one stream for both");
    logs.apply(&key, Logged::Streaming);
    logs.apply(
        &key,
        Logged::Lines(vec![line(9, "ready"), line(10, "job 1")]),
    );
    let held = logs.get(&key).expect("the stream");
    assert_eq!(held.state, Streaming::Open);
    assert_eq!(
        held.buffer().text(),
        "14:02:09.500 ready\n14:02:10.500 job 1\n"
    );
    assert_eq!(held.total, 2);
}

#[test]
fn past_its_cap_a_stream_sheds_its_oldest_lines_and_counts_them_still() {
    let mut logs = Logs::default();
    let key = worker();
    logs.lease(&key, "resource");
    let long = "x".repeat(1024 * 1024);
    for at in 0..12 {
        logs.apply(&key, Logged::Lines(vec![line(at, &long)]));
    }
    let held = logs.get(&key).expect("the stream");
    assert!(held.bytes() <= CAP, "{} bytes", held.bytes());
    assert_eq!(held.total, 12);
    assert_eq!(
        held.lines(),
        held.buffer().lines(),
        "the text sheds the same lines"
    );
    assert!(logs.release("resource").contains(&key));
    logs.drop_unread(&key);
    assert!(logs.get(&key).is_none());
}
