use crate::{Level, LogLine, level, log_time};

#[test]
fn a_kubelet_line_reads_its_time_to_the_nanosecond_and_keeps_its_words() {
    let line = LogLine::read("2026-10-09T14:02:09.1123Z INFO worker ready", None);
    assert_eq!(line.text, "INFO worker ready");
    assert_eq!(line.clock(), "14:02:09.112");
    let (seconds, nanos) = line.at.expect("a time");
    assert_eq!(nanos, 112_300_000);
    assert_eq!(log_time("2026-10-09T14:02:09Z"), Some((seconds, 0)));
    let bare = LogLine::read("no time here", Some("worker".into()));
    assert_eq!((bare.at, bare.text.as_str()), (None, "no time here"));
}

#[test]
fn a_level_is_read_from_the_first_words_in_any_usual_shape() {
    for (said, expected) in [
        ("ERROR paxone.db unreachable", Level::Error),
        ("ts=1 level=warn msg=slow", Level::Warn),
        (r#"{"level":"error","msg":"boom"}"#, Level::Error),
        ("E1009 14:02:09.112 main.go:88] failed", Level::Error),
        ("W1009 14:02:09.112 main.go:88] slow", Level::Warn),
        ("INFO job done, no errors", Level::Other),
        ("Traceback (most recent call last):", Level::Other),
    ] {
        assert_eq!(level(said), expected, "{said}");
    }
}
